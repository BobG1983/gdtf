//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    ecs::system::SystemParam,
    prelude::*,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{situation::Situation, tuning::CombatTuning};
use gdtf_ui::theme::{ActiveThemeHandle, GdtfTheme, GdtfThemeSpec, default_theme};

use crate::scenes::load::resources::{FailedAssetPath, LoadFailed, LoadHandles, LoadedSituation};

/// The three loaded RON asset collections [`poll_and_resolve`] reads, bundled into
/// one [`SystemParam`] so the system's parameter list stays under clippy's
/// argument-count gate (the [`BattleGridsParam`](gdtf_battle_sim) grouping
/// precedent — a transparent bundle of existing world-state resources, not a
/// wrapped domain scalar).
///
/// Each is `Option<Res<…>>` because a `MinimalPlugins` headless app has no
/// `AssetServer` (and so no `Assets<…>` collections); the system early-returns
/// when any is absent, so it never panics on a missing collection (bevy-traps
/// rule 1).
#[derive(SystemParam)]
pub(in crate::scenes::load) struct LoadAssetCollections<'w> {
    /// The loaded theme-spec RON collection (`theme/grimdark.ron`).
    theme:     Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded authored-situation RON collection (`situations/skirmish.ron`).
    situation: Option<Res<'w, Assets<RonAsset<Situation>>>>,
    /// The loaded combat-tuning RON collection (`combat/tuning.ron`, GTW-206).
    tuning:    Option<Res<'w, Assets<RonAsset<CombatTuning>>>>,
}

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
///   the folder is loaded — GTW-149), reads the deserialized [`GdtfThemeSpec`]
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
/// GTW-205 (E10.3): on the success path it ALSO resolves the authored
/// [`Situation`] into a persistent [`LoadedSituation`] — the source the Generation
/// slice (E10.5) reads. Once the situation RON reaches [`LoadState::Loaded`] it is
/// read out of `Assets<RonAsset<Situation>>` (the same transient-one-frame
/// `Assets::get` retry the theme path uses) and inserted as a [`LoadedSituation`]
/// that, like [`GdtfTheme`], persists past `OnExit(Load)`. This is deliberately
/// **non-blocking** to the theme: the situation resolution NEVER gates or alters
/// the existing theme-only Load→Intro transition. A situation that has not (yet)
/// reached `Loaded` simply leaves `LoadedSituation` un-inserted this pass and the
/// theme still resolves — so a slow/failed situation can never strand the machine
/// in `Load`.
///
/// GTW-206 (E10.4): it ALSO resolves the shipped [`CombatTuning`] into a persistent
/// [`CombatTuning`] resource — the balance store the sim marches with. The tuning
/// branch runs on its OWN `CombatTuning`-absence guard ([`resolve_tuning`]), so it
/// neither starves nor is starved by the theme branch: a slow tuning never blocks
/// the theme and a slow theme never blocks the tuning. Unlike the theme it has no
/// `resolve()` step (`CombatTuning` IS both the `Deserialize` payload and the
/// `Resource`), so the loaded payload is inserted directly. On the failure path it
/// `warn!`s naming `combat/tuning.ron` and inserts [`CombatTuning::default`], so
/// `Load` always exits with a tuning present. BOTH a `GdtfTheme` and a
/// `CombatTuning` must be present before the plugin's transition leaves `Load`
/// (see the plugin wiring); this branch makes the tuning the second required
/// resource.
///
/// Guarded by `run_if(resource_exists::<LoadHandles>)` plus the
/// `not(resource_exists::<GdtfTheme>).or(not(resource_exists::<CombatTuning>))`
/// gate in the plugin wiring (run while EITHER required resource is still
/// missing), and takes `Res<AssetServer>`/`Res<Assets<_>>`/`Res<LoadHandles>` —
/// all of which are present whenever those run-conditions hold, so it never panics
/// on a missing resource (bevy-traps rule 1). The early-`return`s on the
/// run-condition resources are belt-and-braces against a one-frame race. The theme
/// branch is internally re-gated on `not(resource_exists::<GdtfTheme>)` so once the
/// theme resolves only the still-missing tuning is polled (and vice-versa).
pub(in crate::scenes::load) fn poll_and_resolve(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: LoadAssetCollections,
    theme_present: Option<Res<GdtfTheme>>,
    tuning_present: Option<Res<CombatTuning>>,
    handles: Option<Res<LoadHandles>>,
) {
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(situation_assets),
        Some(tuning_assets),
        Some(handles),
    ) = (
        asset_server,
        collections.theme,
        collections.situation,
        collections.tuning,
        handles,
    )
    else {
        return;
    };

    // GTW-206 (E10.4): resolve the shipped combat tuning on its OWN absence guard,
    // independently of the theme branch below — so a slow theme never blocks the
    // tuning and a slow tuning never blocks the theme. Done FIRST so it always gets
    // a poll even once the theme has resolved (the system keeps running while
    // EITHER required resource is missing).
    if tuning_present.is_none() {
        resolve_tuning(&mut commands, &asset_server, &tuning_assets, &handles);
    }

    // Once a GdtfTheme exists, the theme branch is done — only the tuning above
    // still needs polling. Skip the theme/situation work to avoid re-resolving it.
    if theme_present.is_some() {
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
        // GTW-205 (E10.3): resolve the authored situation into the persistent
        // LoadedSituation FIRST, on the same success pass. Once it has reached
        // Loaded, read it out of its collection with the same transient-one-frame
        // retry the theme uses (loaded-but-not-yet-in-collection → return, retry
        // next frame). This is NON-BLOCKING to the theme: a situation that has not
        // yet reached Loaded just leaves LoadedSituation un-inserted this pass — so
        // it can never gate or alter the theme-only Load→Intro transition.
        if matches!(
            asset_server.load_state(&*handles.situation),
            LoadState::Loaded
        ) {
            let Some(situation) = situation_assets.get(&*handles.situation) else {
                // Loaded-but-not-yet-in-collection — retry next frame (the theme is
                // not inserted yet, so the run-condition keeps this system alive).
                return;
            };
            // Persist the resolved battlefield for the Generation consumer (E10.5);
            // like GdtfTheme it survives OnExit(Load) (not removed in cleanup).
            commands.insert_resource(LoadedSituation((**situation).clone()));
        }

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
        commands.insert_resource(ActiveThemeHandle((*handles.theme).clone()));
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
    commands.insert_resource(ActiveThemeHandle((*handles.theme).clone()));
}

/// GTW-206 (E10.4): resolves the shipped [`CombatTuning`] RON into the persistent
/// runtime [`CombatTuning`] resource, mirroring the theme path's poll/resolve +
/// warn/fallback shape but for a payload that needs NO `resolve()` step
/// (`CombatTuning` is BOTH the `Deserialize` payload AND the `Resource`).
///
/// Called only while no [`CombatTuning`] resource exists yet (the caller's
/// own-absence guard), independently of the theme branch:
///
/// - If the tuning RON reached [`LoadState::Failed`], `warn!`s naming
///   `combat/tuning.ron` and inserts [`CombatTuning::default`] — the ADR-0003
///   sanctioned error-path safety-net — so `Load` always exits with a tuning
///   present and never hangs on a bad tuning file.
/// - Else once the tuning RON is [`LoadState::Loaded`], reads the deserialized
///   [`CombatTuning`] out of `Assets<RonAsset<CombatTuning>>` (the same
///   transient-one-frame `Assets::get` retry the theme path uses) and inserts the
///   inner payload directly as the persistent resource. Like
///   [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) it survives `OnExit(Load)` (it is
///   **not** removed in `cleanup`), because `BattleScape` reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
fn resolve_tuning(
    commands: &mut Commands,
    asset_server: &AssetServer,
    tuning_assets: &Assets<RonAsset<CombatTuning>>,
    handles: &LoadHandles,
) {
    let tuning_state = asset_server.load_state(&*handles.tuning);

    // Failure path: a bad tuning must not hang the app. Warn naming the path and
    // fall back to the const-default tuning so Load always exits with one present.
    if tuning_state.is_failed() {
        warn!(
            "GDTF Load: asset `combat/tuning.ron` failed to load; falling back to the const \
             default combat tuning",
        );
        commands.insert_resource(CombatTuning::default());
        return;
    }

    // Success path: once the tuning RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert
    // it directly — CombatTuning is both the payload and the runtime resource.
    if matches!(tuning_state, LoadState::Loaded) {
        let Some(tuning) = tuning_assets.get(&*handles.tuning) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while CombatTuning is still absent).
            return;
        };
        commands.insert_resource((**tuning).clone());
    }
}
