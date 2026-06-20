//! The `poll_and_resolve` orchestrator system and the shared theme `fall_back` helper.

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};
use gdtf_ui::theme::{ActiveThemeHandle, GdtfTheme, default_theme};

use crate::scenes::load::{
    resources::{FailedAssetPath, LoadFailed, LoadHandles},
    systems::resolve::{
        armor::resolve_armor,
        params::{LoadAssetCollections, ResolvedResources},
        situation::resolve_situation,
        tuning::resolve_tuning,
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
/// On **both** the success and the failure paths it also inserts the persistent
/// [`ActiveThemeHandle`] (the theme RON handle from [`LoadHandles`]) alongside the
/// [`GdtfTheme`] — the handle is valid even when the load failed, so GTW-138's
/// later file-watcher reload can recover, and the GTW-137 live-retheme system
/// filters incoming asset events against it. Holding it keeps a strong reference
/// so the asset stays loaded for that watcher. Like [`GdtfTheme`], it persists
/// past `OnExit(Load)` (it is **not** removed in `cleanup`).
///
/// GTW-205 (E10.3) / GTW-261: it ALSO resolves the authored
/// [`Situation`](gdtf_battle_sim::situation::Situation) into a persistent
/// [`LoadedSituation`](crate::scenes::load::resources::LoadedSituation) — the source
/// the Generation slice (E10.5) reads. As of GTW-261 the situation is a
/// **gate-blocking** resource (the
/// [empty-battle-race fix](resolve_situation)): the Load→Intro transition now
/// requires a `LoadedSituation` too, so a battle never starts before its real
/// situation loads. The situation branch runs on its OWN `LoadedSituation`-absence
/// guard ([`resolve_situation`]), exactly like the tuning and weapons branches, so a
/// slow theme never blocks the situation and vice-versa. On the failure path it
/// `warn!`s and inserts an empty
/// [`Situation::default`](gdtf_battle_sim::situation::Situation::default), preserving
/// the no-strand guarantee (a slow/failed situation still always lets `Load` exit),
/// while a success resolves the real authored battlefield.
///
/// GTW-206 (E10.4): it ALSO resolves the shipped
/// [`CombatTuning`](gdtf_battle_sim::tuning::CombatTuning) into a persistent
/// `CombatTuning` resource — the balance store the sim marches with. The tuning
/// branch runs on its OWN `CombatTuning`-absence guard ([`resolve_tuning`]), so it
/// neither starves nor is starved by the theme branch: a slow tuning never blocks
/// the theme and a slow theme never blocks the tuning. Unlike the theme it has no
/// `resolve()` step (`CombatTuning` IS both the `Deserialize` payload and the
/// `Resource`), so the loaded payload is inserted directly. On the failure path it
/// `warn!`s naming `combat/tuning.ron` and inserts `CombatTuning::default`, so
/// `Load` always exits with a tuning present. A `GdtfTheme`, a `CombatTuning`, a
/// `WeaponRegistry`, a `LoadedSituation`, AND an `ArmorRegistry` must ALL be present
/// before the plugin's transition leaves `Load` (see the plugin wiring); this branch
/// makes the tuning one of those five required resources.
///
/// GTW-269: it ALSO resolves the loaded `assets/armor/` folder into a persistent
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) (the armor mirror of the
/// `WeaponRegistry` branch). The armor branch runs on its OWN `ArmorRegistry`-absence
/// guard ([`resolve_armor`]), so it neither starves nor is starved by the other
/// branches. The registry is DORMANT after this slice — nothing consumes it yet
/// (slice C does); it is resolved and gated on purely so it is present (and the
/// folder verified loaded) before `Load` exits. On the failure path
/// [`resolve_armor`] `warn!`s and inserts an empty registry, preserving the
/// no-strand guarantee.
///
/// Guarded by `run_if(resource_exists::<LoadHandles>)` plus the
/// `not(resource_exists::<GdtfTheme>).or(not(resource_exists::<CombatTuning>))
/// .or(not(resource_exists::<WeaponRegistry>)).or(not(resource_exists::<LoadedSituation>))
/// .or(not(resource_exists::<ArmorRegistry>))`
/// gate in the plugin wiring (run while ANY of the five required resources is still
/// missing), and takes `Res<AssetServer>`/`Res<Assets<_>>`/`Res<LoadHandles>` —
/// all of which are present whenever those run-conditions hold, so it never panics
/// on a missing resource (bevy-traps rule 1). The early-`return`s on the
/// run-condition resources are belt-and-braces against a one-frame race. Each branch
/// is internally re-gated on its OWN resource's absence (via the
/// [`ResolvedResources`] presence-probes) so once one resolves only the still-missing
/// ones keep being polled.
pub(in crate::scenes::load) fn poll_and_resolve(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: LoadAssetCollections,
    resolved: ResolvedResources,
    handles: Option<Res<LoadHandles>>,
) {
    let (theme_present, tuning_present, weapons_present, situation_present, armor_present) = (
        resolved.theme.is_some(),
        resolved.tuning.is_some(),
        resolved.weapons.is_some(),
        resolved.situation.is_some(),
        resolved.armor.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(situation_assets),
        Some(tuning_assets),
        Some(folders),
        Some(weapon_specs),
        Some(armor_specs),
        Some(handles),
    ) = (
        asset_server,
        collections.theme,
        collections.situation,
        collections.tuning,
        collections.folders,
        collections.weapon_specs,
        collections.armor_specs,
        handles,
    )
    else {
        return;
    };

    // GTW-206 (E10.4): resolve the shipped combat tuning on its OWN absence guard,
    // independently of the theme branch below — so a slow theme never blocks the
    // tuning and a slow tuning never blocks the theme. Done FIRST so it always gets
    // a poll even once the theme has resolved (the system keeps running while
    // ANY required resource is missing).
    if !tuning_present {
        resolve_tuning(&mut commands, &asset_server, &tuning_assets, &handles);
    }

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

    // GTW-261: resolve the authored situation into the persistent LoadedSituation on
    // its OWN absence guard, independently of the theme/tuning/weapons branches — so a
    // slow theme never blocks the situation and vice-versa (the tuning-branch
    // precedent). This is the empty-battle-race fix: the situation is now a
    // gate-blocking resource (see the plugin wiring), resolved here on success and
    // falling back to an empty default on failure (so Load never strands).
    if !situation_present {
        resolve_situation(&mut commands, &asset_server, &situation_assets, &handles);
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
            FailedAssetPath(String::from("theme/grimdark.ron")),
            &handles,
        );
        return;
    }
    if matches!(fonts_state, RecursiveDependencyLoadState::Failed(_)) {
        fall_back(
            &mut commands,
            FailedAssetPath(String::from("fonts")),
            &handles,
        );
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
        let theme: GdtfTheme = (**spec)
            .clone()
            .resolve(|key| asset_server.load::<Font>(key.to_owned()));
        commands.insert_resource(theme);
        // The persistent handle the GTW-137 retheme system filters against and
        // GTW-138's watcher keeps loaded; survives OnExit(Load) like GdtfTheme.
        commands.insert_resource(ActiveThemeHandle::new((*handles.theme).clone()));
    }
}

/// Records the failed asset path, warns naming it, and inserts the const-fallback
/// [`GdtfTheme`] plus the persistent [`ActiveThemeHandle`].
///
/// Shared by both failure branches so the warn-and-fallback is written once. The
/// [`ActiveThemeHandle`] is inserted on this path too: the handle is valid even
/// though the load failed, so GTW-138's file-watcher reload can recover from it.
fn fall_back(commands: &mut Commands, path: FailedAssetPath, handles: &LoadHandles) {
    warn!(
        "GDTF Load: asset `{}` failed to load; falling back to the const default theme",
        &*path,
    );
    commands.insert_resource(LoadFailed(path));
    commands.insert_resource(default_theme());
    commands.insert_resource(ActiveThemeHandle::new((*handles.theme).clone()));
}
