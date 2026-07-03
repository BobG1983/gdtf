//! The `poll_and_resolve` orchestrator system and the shared theme `fall_back` helper.

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};
use gdtf_ui::{
    resolve_theme_spec,
    theme::{GdtfTheme, default_theme},
};

use crate::states::load::{
    resources::{FailedAssetPath, LoadFailed, LoadHandles},
    systems::resolve::{
        attachments::resolve_attachments,
        injuries::resolve_injuries,
        params::{LoadAssetCollections, ResolvedResources},
        prefab::resolve_prefabs,
    },
};

/// Polls the in-flight loads and, once resolvable, inserts the [`GdtfTheme`].
///
/// Each frame, while [`LoadHandles`] exists and no [`GdtfTheme`] has been
/// inserted yet:
///
/// - If the theme RON reached [`LoadState::Failed`] **or** the fonts folder
///   reached [`RecursiveDependencyLoadState::Failed`], records the failed path in
///   a typed [`LoadFailed`] resource, `warn!`s naming it, and inserts the
///   const-fallback [`default_theme`] — the ADR-0003 sanctioned error-path
///   safety-net — so the app never hangs and never leaves `Load` themeless.
/// - Else once the theme RON is [`LoadState::Loaded`] **and** the fonts folder's
///   [`RecursiveDependencyLoadState`] is `Loaded` (recursive, so every font in
///   the folder is loaded — GTW-149), reads the deserialized
///   [`GdtfThemeSpec`](gdtf_ui::theme::GdtfThemeSpec)
///   out of `Assets<RonAsset<GdtfThemeSpec>>` and resolves it with the font
///   resolver `|key| asset_server.load::<Font>(key)` into a [`GdtfTheme`], then
///   inserts it. `load` is idempotent — each font key returns its
///   already-preloaded handle.
/// - Else (still loading) it does nothing and runs again next frame.
///
/// On **both** the success and the failure paths it also inserts the theme's
/// persistent generic [`HotRonHandle`](gdtf_assets::HotRonHandle) (the theme RON
/// handle from [`LoadHandles`]) alongside the [`GdtfTheme`] — the handle is valid
/// even when the load failed, so GTW-138's later file-watcher reload can recover,
/// and the GTW-137 live-retheme redrive (the GTW-564 generic one `UiPlugin`
/// registers) filters incoming asset events against it. Holding it keeps a strong
/// reference so the asset stays loaded for that watcher. Like [`GdtfTheme`], it
/// persists past `OnExit(Load)` (it is **not** removed in `cleanup`).
///
/// GTW-564: the SITUATION and the COMBAT / STAT / PROCGEN tuning branches left
/// this orchestrator — each is now a generic hot-RON chain (one ext call in the
/// Load plugin). GTW-570: the seven FOLDER content families (ranged/melee
/// weapons, armor, fields, gangs, terrain + theme defs) left it the same way —
/// each is now a generic content-family chain whose gated resolve publishes the
/// SAME gate-blocking registry with the SAME never-publish-partial +
/// genuine-`Failed`-only empty-registry semantics; the `transition_to_intro`
/// gate chain still requires every one of them, unchanged.
///
/// The BESPOKE folder branches that remain — the declared GTW-570 exclusions —
/// each run on their OWN absence guard, so none starves another:
///
/// - GTW-549: [`resolve_attachments`] builds the name-keyed
///   [`AttachmentRegistry`](gdtf_battle_sim::weapon::AttachmentRegistry)
///   (adoption of the generic seam belongs to the GTW-579 rider).
/// - GTW-437: [`resolve_injuries`] builds BOTH the
///   [`InjuryRegistry`](gdtf_battle_sim::injuries::InjuryRegistry) and the
///   [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables) from ONE folder.
/// - GTW-489: [`resolve_prefabs`] builds the UUID-keyed
///   [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) multimap.
///
/// On each failure path the branch `warn!`s and inserts an empty registry,
/// preserving the no-strand guarantee.
///
/// Guarded by `run_if(resource_exists::<LoadHandles>)` plus the per-resource
/// `not(resource_exists::<…>)` or-chain in the plugin wiring (run while ANY required
/// resource is still missing), and takes `Res<AssetServer>`/`Res<Assets<_>>`/`Res<LoadHandles>` —
/// all of which are present whenever those run-conditions hold, so it never panics
/// on a missing resource (bevy-traps rule 1). The early-`return`s on the
/// run-condition resources are belt-and-braces against a one-frame race. Each branch
/// is internally re-gated on its OWN resource's absence (via the
/// [`ResolvedResources`] presence-probes) so once one resolves only the still-missing
/// ones keep being polled.
pub(in crate::states::load) fn poll_and_resolve(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: LoadAssetCollections,
    resolved: ResolvedResources,
    handles: Option<Res<LoadHandles>>,
) {
    let (theme_present, attachments_present, injuries_present, prefabs_present) = (
        resolved.theme.is_some(),
        resolved.attachments.is_some(),
        resolved.injuries.is_some(),
        resolved.prefabs.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(folders),
        Some(attachment_specs),
        Some(injury_defs),
        Some(weightings),
        Some(prefab_specs),
        Some(handles),
    ) = (
        asset_server,
        collections.theme,
        collections.folders,
        collections.attachment_specs,
        collections.injury_defs,
        collections.weightings,
        collections.prefab_specs,
        handles,
    )
    else {
        return;
    };

    // GTW-549 PHASE 1: resolve the attachments folder into the name-keyed AttachmentRegistry on
    // its OWN absence guard, independently of all other branches. PHASE 2 resolves each
    // weapon's `attachment_slots` keys against it. On the failure path resolve_attachments
    // warn!s and inserts an empty registry, preserving the no-strand guarantee.
    if !attachments_present {
        resolve_attachments(
            &mut commands,
            &asset_server,
            &folders,
            &attachment_specs,
            &handles,
        );
    }

    // GTW-437: resolve the injuries folder into the InjuryRegistry + InjuryTables on
    // its OWN absence guard (the registry's presence is the branch done-probe; the
    // tables are inserted in the same branch), independently of all other branches —
    // so a slow injuries folder never blocks them and vice-versa.
    if !injuries_present {
        resolve_injuries(
            &mut commands,
            &asset_server,
            &folders,
            &injury_defs,
            &weightings,
            &handles,
        );
    }

    // GTW-489: resolve the nested `assets/content/maps/` folder into the bucketed PrefabRegistry
    // on its OWN absence guard, independently of all other branches. Fragments load from the
    // `content/maps/<theme>/<size>/` tree via the dedicated `prefab.ron` extension, with NO
    // edge-opening validation — an openingless (zero-placement) prefab is INCLUDED (the
    // GTW-488 design). The procgen pipeline consumes the registry (GTW-492); a failed folder
    // resolve falls back to an empty registry, preserving the no-strand guarantee.
    if !prefabs_present {
        resolve_prefabs(
            &mut commands,
            &asset_server,
            &folders,
            &prefab_specs,
            &handles,
        );
    }

    // Once a GdtfTheme exists, the theme branch is done — only the branches above
    // still need polling. Skip the theme work to avoid re-resolving it.
    if theme_present {
        return;
    }

    let theme_state = asset_server.load_state(&*handles.theme);
    // Recursive (not direct) — gate on every font IN the folder being loaded.
    let fonts_state = asset_server.recursive_dependency_load_state(&*handles.fonts);

    // Failure path: a failed required asset must not hang the app. Record the
    // failed path, warn, and fall back to the const default theme. The active
    // theme handle is inserted even here — it is valid despite the failed load,
    // so a later file-watcher reload (GTW-138) can recover from it.
    if theme_state.is_failed() {
        fall_back(
            &mut commands,
            FailedAssetPath::new("core_tuning/ui_theme.tuning.ron"),
            &handles,
        );
        return;
    }
    if matches!(fonts_state, RecursiveDependencyLoadState::Failed(_)) {
        fall_back(&mut commands, FailedAssetPath::new("fonts"), &handles);
        return;
    }

    // Success path: the theme RON is loaded AND every font in the folder is
    // loaded — resolve the spec, loading each font key idempotently.
    if matches!(theme_state, LoadState::Loaded)
        && matches!(fonts_state, RecursiveDependencyLoadState::Loaded)
    {
        let Some(spec) = theme_assets.get(&*handles.theme) else {
            // Loaded-but-not-yet-in-collection is a transient one-frame state;
            // try again next frame rather than failing.
            return;
        };
        // Resolve through the ONE shared map hook (gdtf_ui::resolve_theme_spec) —
        // the same fn the generic redrive re-runs on a hot edit, so a live retheme
        // yields exactly what this first resolve did.
        let theme: GdtfTheme = resolve_theme_spec(spec, &asset_server);
        commands.insert_resource(theme);
        // The persistent generic handle the GTW-137 retheme redrive filters against
        // and GTW-138's watcher keeps loaded; survives OnExit(Load) like GdtfTheme.
        commands.insert_resource(handles.theme.clone());
    }
}

/// Records the failed asset path, warns naming it, and inserts the const-fallback
/// [`GdtfTheme`] plus the theme's persistent generic
/// [`HotRonHandle`](gdtf_assets::HotRonHandle).
///
/// Shared by both failure branches so the warn-and-fallback is written once. The
/// handle is inserted on this path too: it is valid even though the load failed,
/// so GTW-138's file-watcher reload can recover from it.
fn fall_back(commands: &mut Commands, path: FailedAssetPath, handles: &LoadHandles) {
    warn!(
        "GDTF Load: asset `{}` failed to load; falling back to the const default theme",
        &*path,
    );
    commands.insert_resource(LoadFailed::new(path));
    commands.insert_resource(default_theme());
    commands.insert_resource(handles.theme.clone());
}
