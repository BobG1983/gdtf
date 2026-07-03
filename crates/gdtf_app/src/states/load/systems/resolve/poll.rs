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
        armor::resolve_armor,
        attachments::resolve_attachments,
        fields::resolve_fields,
        gangs::resolve_gangs,
        injuries::resolve_injuries,
        melee_weapons::resolve_melee_weapons,
        params::{LoadAssetCollections, ResolvedResources},
        prefab::resolve_prefabs,
        terrain_model::{resolve_terrain_defs, resolve_theme_defs},
        weapons::resolve_weapons,
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
/// Load plugin) whose gated resolve publishes the SAME gate-blocking resource
/// (`LoadedSituation` / `CombatTuning` / `GangerStatTuning` / `ProcgenTuning`)
/// with the SAME never-publish-partial + genuine-`Failed`-only fallback
/// semantics; the `transition_to_intro` gate chain still requires every one of
/// them, unchanged.
///
/// GTW-269: it ALSO resolves the loaded `assets/content/armor/` folder into a persistent
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) (the armor mirror of the
/// `WeaponRegistry` branch). The armor branch runs on its OWN `ArmorRegistry`-absence
/// guard ([`resolve_armor`]), so it neither starves nor is starved by the other
/// branches. The registry is DORMANT after this slice — nothing consumes it yet
/// (slice C does); it is resolved and gated on purely so it is present (and the
/// folder verified loaded) before `Load` exits. On the failure path
/// [`resolve_armor`] `warn!`s and inserts an empty registry, preserving the
/// no-strand guarantee.
///
/// GTW-487 / GTW-489 / GTW-494: it ALSO resolves the per-theme `terrain/` folder into the
/// UUID-keyed [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry), and the `maps/` folder
/// into the UUID-keyed [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry), each on
/// its OWN absence guard ([`resolve_terrain_defs`] / [`resolve_theme_defs`] /
/// [`resolve_prefabs`]), so none starves another. GTW-494 RETIRED the old flat-dir
/// `terrain` / `themes` / `maps` resolvers — these UUID-model loaders are now the ONLY
/// terrain / theme / prefab resolvers (the sim + procgen + presenter consume the new
/// registries as of GTW-491/492/493). Each is resolved and gated on so the folder is
/// verified loaded before `Load` exits; a failed folder resolve falls back to an empty
/// registry, preserving the no-strand guarantee.
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
// GTW-394 pushed this function over the 100-line clippy limit (each branch adds ~15
// lines of resolve work). The function is a flat, linearly-growing orchestrator whose
// shape is self-evidently correct at a glance — splitting it would just separate the
// closely-related branch logic without reducing conceptual load.
#[allow(
    clippy::too_many_lines,
    reason = "flat per-resource branch orchestrator; each GTW adds ~15 lines of resolve \
              work; splitting would separate closely-related logic without reducing load"
)]
pub(in crate::states::load) fn poll_and_resolve(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: LoadAssetCollections,
    resolved: ResolvedResources,
    handles: Option<Res<LoadHandles>>,
) {
    let (
        theme_present,
        weapons_present,
        melee_weapons_present,
        attachments_present,
        armor_present,
        fields_present,
        injuries_present,
        gangs_present,
        prefabs_present,
        terrain_defs_present,
        theme_defs_present,
    ) = (
        resolved.theme.is_some(),
        resolved.weapons.is_some(),
        resolved.melee_weapons.is_some(),
        resolved.attachments.is_some(),
        resolved.armor.is_some(),
        resolved.fields.is_some(),
        resolved.injuries.is_some(),
        resolved.gangs.is_some(),
        resolved.prefabs.is_some(),
        resolved.terrain_defs.is_some(),
        resolved.theme_defs.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(folders),
        Some(weapon_specs),
        Some(melee_specs),
        Some(attachment_specs),
        Some(armor_specs),
        Some(field_defs),
        Some(injury_defs),
        Some(weightings),
        Some(gang_rosters),
        Some(prefab_specs),
        Some(terrain_defs),
        Some(theme_defs),
        Some(handles),
    ) = (
        asset_server,
        collections.theme,
        collections.folders,
        collections.weapon_specs,
        collections.melee_specs,
        collections.attachment_specs,
        collections.armor_specs,
        collections.field_defs,
        collections.injury_defs,
        collections.weightings,
        collections.gang_rosters,
        collections.prefab_specs,
        collections.terrain_defs,
        collections.theme_defs,
        handles,
    )
    else {
        return;
    };

    // GTW-257: resolve the weapons folder into the name-keyed WeaponRegistry on its
    // OWN absence guard, independently of the theme/tuning branches — so a slow
    // weapons folder never blocks them and vice-versa (the tuning-branch precedent).
    if !weapons_present {
        resolve_weapons(
            &mut commands,
            &asset_server,
            &folders,
            &weapon_specs,
            &handles,
        );
    }

    // GTW-505: resolve the MELEE weapons folder into the name-keyed MeleeWeaponRegistry on
    // its OWN absence guard, independently of all other branches (the ranged-weapons-branch
    // precedent). The setup resolves each ganger's melee weapon against it (an authored key
    // or the `fists` default). On the failure path resolve_melee_weapons warn!s and inserts
    // an empty registry, preserving the no-strand guarantee.
    if !melee_weapons_present {
        resolve_melee_weapons(
            &mut commands,
            &asset_server,
            &folders,
            &melee_specs,
            &handles,
        );
    }

    // GTW-549 PHASE 1: resolve the attachments folder into the name-keyed AttachmentRegistry on
    // its OWN absence guard, independently of all other branches (the melee-weapons-branch
    // precedent). PHASE 2 resolves each weapon's `attachment_slots` keys against it. On the
    // failure path resolve_attachments warn!s and inserts an empty registry, preserving the
    // no-strand guarantee.
    if !attachments_present {
        resolve_attachments(
            &mut commands,
            &asset_server,
            &folders,
            &attachment_specs,
            &handles,
        );
    }

    // GTW-269: resolve the armor folder into the name-keyed ArmorRegistry on its OWN
    // absence guard, independently of the theme/tuning/weapons/situation branches —
    // so a slow armor folder never blocks them and vice-versa (the weapons-branch
    // precedent). The registry is DORMANT after this slice (nothing consumes it yet —
    // slice C does); it is resolved here purely so it is present when the gate checks.
    if !armor_present {
        resolve_armor(
            &mut commands,
            &asset_server,
            &folders,
            &armor_specs,
            &handles,
        );
    }

    // GTW-545: resolve the area-damage-fields folder into the stem-keyed FieldDefRegistry on
    // its OWN absence guard, independently of all other branches (the armor-branch precedent).
    // The battle setup resolves each situation's authored `fields:` placement against it. On
    // the failure path resolve_fields warn!s and inserts an empty registry, preserving the
    // no-strand guarantee.
    if !fields_present {
        resolve_fields(
            &mut commands,
            &asset_server,
            &folders,
            &field_defs,
            &handles,
        );
    }

    // GTW-437: resolve the injuries folder into the InjuryRegistry + InjuryTables on
    // its OWN absence guard (the registry's presence is the branch done-probe; the
    // tables are inserted in the same branch), independently of all other branches —
    // so a slow injuries folder never blocks them and vice-versa (the armor-branch
    // precedent). The resources are DORMANT after this slice (the GTW-438 roll consumes
    // them); they are resolved here purely so they are present when the gate checks.
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

    // GTW-415: resolve the gangs folder into the name-keyed GangRegistry on its OWN
    // absence guard, independently of all other branches — so a slow gangs folder never
    // blocks them and vice-versa (the weapons-branch precedent). This is the GTW-414/415
    // feature-completeness piece: WITHOUT it the GangRegistry is never populated from the
    // shipped `assets/content/gangs/` files, and every real battle fails closed with
    // GangNotFound (the placed-ganger gang/member refs cannot resolve). On the failure
    // path resolve_gangs warn!s and inserts an empty registry, preserving the no-strand
    // guarantee.
    if !gangs_present {
        resolve_gangs(
            &mut commands,
            &asset_server,
            &folders,
            &gang_rosters,
            &handles,
        );
    }

    // GTW-489: resolve the nested `assets/content/maps/` folder into the bucketed PrefabRegistry
    // on its OWN absence guard, independently of all other branches (the gangs-branch precedent).
    // Fragments load from the `content/maps/<theme>/<size>/` tree via the dedicated `prefab.ron`
    // extension. UNLIKE the retired flat-dir prefab
    // resolve, this build runs NO C6 edge-opening validation — an openingless (zero-placement)
    // prefab is INCLUDED (the GTW-488 design). The procgen pipeline consumes the registry
    // (GTW-492); it is resolved and gated on so the folder is verified loaded before `Load`
    // exits. A failed folder resolve falls back to an empty registry, preserving the no-strand
    // guarantee.
    if !prefabs_present {
        resolve_prefabs(
            &mut commands,
            &asset_server,
            &folders,
            &prefab_specs,
            &handles,
        );
    }

    // GTW-487: resolve the NEW per-theme `terrain/` folder into the UUID-keyed
    // TerrainDefRegistry + UuidThemeRegistry, each on its OWN absence guard, independently
    // of all other branches (the terrain/themes-branch precedent). These run BESIDE the
    // legacy terrain/themes branches above — the new model loads from a DISTINCT
    // `terrain/<theme>/` layout via dedicated `terrain_def.ron` / `terrain_theme.ron`
    // extensions, so neither collides with the legacy `terrain.ron` / `theme.ron` loaders.
    // Both registries are DORMANT after this slice — nothing consumes them yet (later
    // GTW-476 switch tickets do); they are resolved and gated on purely so the new folder
    // is verified loaded before `Load` exits. They resolve EMPTY against un-migrated
    // shipped content (the designed fail-closed state, not a failure); a failed folder
    // resolve falls back to empty registries, preserving the no-strand guarantee.
    if !terrain_defs_present {
        resolve_terrain_defs(
            &mut commands,
            &asset_server,
            &folders,
            &terrain_defs,
            &handles,
        );
    }
    if !theme_defs_present {
        resolve_theme_defs(
            &mut commands,
            &asset_server,
            &folders,
            &theme_defs,
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
