//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.

use bevy::{
    asset::{LoadState, LoadedFolder, RecursiveDependencyLoadState},
    ecs::system::SystemParam,
    prelude::*,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    situation::Situation,
    tuning::CombatTuning,
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
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
    theme:        Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded authored-situation RON collection (`situations/skirmish.ron`).
    situation:    Option<Res<'w, Assets<RonAsset<Situation>>>>,
    /// The loaded combat-tuning RON collection (`combat/tuning.ron`, GTW-206).
    tuning:       Option<Res<'w, Assets<RonAsset<CombatTuning>>>>,
    /// The loaded `LoadedFolder` collection — used to read the weapons folder's
    /// member handles when building the [`WeaponRegistry`] (GTW-257).
    folders:      Option<Res<'w, Assets<LoadedFolder>>>,
    /// The loaded per-weapon RON collection (`weapons/*.ron`, GTW-257).
    weapon_specs: Option<Res<'w, Assets<RonAsset<WeaponSpec>>>>,
}

/// The four persistent resources [`poll_and_resolve`] resolves, each as an
/// `Option<Res<…>>` presence-probe, bundled into one [`SystemParam`] so the
/// system's parameter list stays under clippy's argument-count gate (the
/// [`LoadAssetCollections`] grouping precedent — a transparent bundle of existing
/// world-state resources, not a wrapped domain scalar).
///
/// Each branch resolves on its OWN resource's absence (so none starves another,
/// bevy-traps rule 3); the bundle exposes that per-branch "already present?" probe.
#[derive(SystemParam)]
pub(in crate::scenes::load) struct ResolvedResources<'w> {
    /// Whether the resolved [`GdtfTheme`] is already inserted.
    theme:     Option<Res<'w, GdtfTheme>>,
    /// Whether the resolved [`CombatTuning`] is already inserted (GTW-206).
    tuning:    Option<Res<'w, CombatTuning>>,
    /// Whether the resolved [`WeaponRegistry`] is already inserted (GTW-257).
    weapons:   Option<Res<'w, WeaponRegistry>>,
    /// Whether the resolved [`LoadedSituation`] is already inserted (GTW-261).
    situation: Option<Res<'w, LoadedSituation>>,
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
/// GTW-205 (E10.3) / GTW-261: it ALSO resolves the authored [`Situation`] into a
/// persistent [`LoadedSituation`] — the source the Generation slice (E10.5) reads.
/// As of GTW-261 the situation is a **gate-blocking** resource (the
/// [empty-battle-race fix](resolve_situation)): the Load→Intro transition now
/// requires a `LoadedSituation` too, so a battle never starts before its real
/// situation loads. The situation branch runs on its OWN `LoadedSituation`-absence
/// guard ([`resolve_situation`]), exactly like the tuning and weapons branches, so a
/// slow theme never blocks the situation and vice-versa. On the failure path it
/// `warn!`s and inserts an empty [`Situation::default`], preserving the no-strand
/// guarantee (a slow/failed situation still always lets `Load` exit), while a
/// success resolves the real authored battlefield.
///
/// GTW-206 (E10.4): it ALSO resolves the shipped [`CombatTuning`] into a persistent
/// [`CombatTuning`] resource — the balance store the sim marches with. The tuning
/// branch runs on its OWN `CombatTuning`-absence guard ([`resolve_tuning`]), so it
/// neither starves nor is starved by the theme branch: a slow tuning never blocks
/// the theme and a slow theme never blocks the tuning. Unlike the theme it has no
/// `resolve()` step (`CombatTuning` IS both the `Deserialize` payload and the
/// `Resource`), so the loaded payload is inserted directly. On the failure path it
/// `warn!`s naming `combat/tuning.ron` and inserts [`CombatTuning::default`], so
/// `Load` always exits with a tuning present. A `GdtfTheme`, a `CombatTuning`, a
/// `WeaponRegistry`, AND a `LoadedSituation` must ALL be present before the plugin's
/// transition leaves `Load` (see the plugin wiring); this branch makes the tuning
/// one of those four required resources.
///
/// Guarded by `run_if(resource_exists::<LoadHandles>)` plus the
/// `not(resource_exists::<GdtfTheme>).or(not(resource_exists::<CombatTuning>))
/// .or(not(resource_exists::<WeaponRegistry>)).or(not(resource_exists::<LoadedSituation>))`
/// gate in the plugin wiring (run while ANY of the four required resources is still
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
    let (theme_present, tuning_present, weapons_present, situation_present) = (
        resolved.theme.is_some(),
        resolved.tuning.is_some(),
        resolved.weapons.is_some(),
        resolved.situation.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(situation_assets),
        Some(tuning_assets),
        Some(folders),
        Some(weapon_specs),
        Some(handles),
    ) = (
        asset_server,
        collections.theme,
        collections.situation,
        collections.tuning,
        collections.folders,
        collections.weapon_specs,
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

/// GTW-205 / GTW-261: resolves the authored [`Situation`] RON into the persistent
/// [`LoadedSituation`] resource, mirroring the [`resolve_tuning`] poll/resolve +
/// warn/fallback shape.
///
/// As of GTW-261 the situation is a **gate-blocking** resource (the empty-battle-race
/// fix): the Load→Intro transition now requires a `LoadedSituation` (see the plugin
/// wiring), so a battle never starts before its real situation loads. This resolve
/// makes the situation exactly symmetric with the tuning and weapons branches — it
/// gates entry AND falls back to an empty default on failure, so a slow/failed
/// situation still always lets `Load` exit (the no-strand guarantee GTW-205 wanted,
/// preserved via the failure fallback rather than via non-blocking resolution).
///
/// Called only while no [`LoadedSituation`] resource exists yet (the caller's
/// own-absence guard), independently of the theme / tuning / weapons branches:
///
/// - If the situation RON reached [`LoadState::Failed`], `warn!`s naming
///   `situations/skirmish.ron` and inserts an empty [`Situation::default`] — the
///   ADR-0003 sanctioned error-path safety-net — so `Load` always exits with a
///   situation present and never hangs on a bad situation file. CRITICAL: the empty
///   default is inserted ONLY on a genuine `Failed`, NEVER while the situation is
///   still loading — inserting it early would clear the gate before the real
///   battlefield resolves, re-introducing the empty-battle bug.
/// - Else once the situation RON is [`LoadState::Loaded`], reads the deserialized
///   [`Situation`] out of `Assets<RonAsset<Situation>>` (the same transient-one-frame
///   `Assets::get` retry the theme path uses) and inserts the inner payload as the
///   persistent [`LoadedSituation`]. Like
///   [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) it survives `OnExit(Load)` (it is
///   **not** removed in `cleanup`), because the Generation slice (E10.5) reads it.
/// - Else (still loading) it does nothing and is polled again next frame.
fn resolve_situation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    situation_assets: &Assets<RonAsset<Situation>>,
    handles: &LoadHandles,
) {
    let situation_state = asset_server.load_state(&*handles.situation);

    // Failure path: a bad/missing situation must not hang the app. Warn naming the
    // path and fall back to the empty default situation so Load always exits with one
    // present (a Failed → empty default; the battle then has zero gangers rather than
    // stranding the machine). Only on a genuine Failed — never while still loading.
    if situation_state.is_failed() {
        warn!(
            "GDTF Load: asset `situations/skirmish.ron` failed to load; falling back to the empty \
             default situation (the battle will have no gangers)",
        );
        commands.insert_resource(LoadedSituation(Situation::default()));
        return;
    }

    // Success path: once the situation RON is loaded, read the deserialized payload
    // out of its collection (transient-one-frame retry like the theme) and insert it
    // as the persistent LoadedSituation.
    if matches!(situation_state, LoadState::Loaded) {
        let Some(situation) = situation_assets.get(&*handles.situation) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while LoadedSituation is still absent).
            return;
        };
        // Persist the resolved battlefield for the Generation consumer (E10.5);
        // like GdtfTheme it survives OnExit(Load) (not removed in cleanup).
        commands.insert_resource(LoadedSituation((**situation).clone()));
    }
}

/// GTW-257: builds the name-keyed [`WeaponRegistry`] from the loaded
/// `assets/weapons/` folder, mirroring the fonts folder-load gating + the situation
/// resolve shape.
///
/// Called only while no [`WeaponRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the theme / tuning branches:
///
/// - Gates on the weapons folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every weapon `.ron`
///   IN the folder is loaded — the fonts-folder pattern). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`WeaponRegistry`] so `Load` always exits with one present and never hangs on a
///   bad folder (the ADR-0003 error-path safety-net; a battle then fails closed with
///   [`WeaponNotFound`](gdtf_battle_sim::situation::BattleSetupError) rather than
///   crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<WeaponSpec>`, reads its [`WeaponSpec`] out of the
///   `Assets<RonAsset<WeaponSpec>>` collection, keys it by the asset path's file
///   STEM with the dedicated `.weapon` infix stripped (so `autogun.weapon.ron` keys
///   `autogun` — the weapon KEY), and inserts every `(WeaponName, WeaponSpec)` into
///   the registry. If ANY member spec is not yet in the collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries
///   next frame — so a partial / empty registry is never published while the folder
///   is non-empty. The registry holds the specs BY VALUE, so they survive the folder
///   handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
fn resolve_weapons(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    weapon_specs: &Assets<RonAsset<WeaponSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.weapons);

    // Failure path: a bad/missing weapons folder must not hang the app. Warn and
    // insert an EMPTY registry so Load always exits with one present (a battle then
    // fails closed with WeaponNotFound rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `weapons` folder failed to load; inserting an empty WeaponRegistry \
             (battles will fail closed on a missing weapon key)",
        );
        commands.insert_resource(WeaponRegistry::default());
        return;
    }

    // Success path: once every weapon file in the folder is loaded, read the
    // LoadedFolder's member handles and build the name-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(folder) = folders.get(&*handles.weapons) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while the WeaponRegistry is still absent).
            return;
        };

        let mut registry = WeaponRegistry::default();
        for untyped in &folder.handles {
            // Type the untyped member handle as a RonAsset<WeaponSpec> and read its
            // spec out of the collection.
            let handle = untyped
                .clone()
                .typed_debug_checked::<RonAsset<WeaponSpec>>();
            let Some(spec) = weapon_specs.get(&handle) else {
                // One-frame loaded-but-not-yet-in-collection race: a member spec is not
                // in the collection yet. RETURN (do NOT insert a partial registry) so we
                // re-poll next frame — the WeaponRegistry stays absent, keeping this
                // branch alive until every member resolves.
                return;
            };
            // Key by the asset path's file STEM with the dedicated `.weapon` infix
            // stripped: `autogun.weapon.ron`'s `file_stem()` is `autogun.weapon`, whose
            // weapon KEY is `autogun`. A handle with no resolvable path / stem is
            // skipped defensively (it would carry no usable key).
            let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
                path.path()
                    .file_stem()
                    .map(|stem| weapon_key_from_stem(&stem.to_string_lossy()))
            }) else {
                continue;
            };
            registry.insert(WeaponName::new(stem), (**spec).clone());
        }

        // Insert the built registry — like GdtfTheme / CombatTuning it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the battle reads it.
        commands.insert_resource(registry);
    }
}

/// The weapon KEY for a loaded weapon file's stem — the stem with the dedicated
/// `.weapon` infix stripped (GTW-257).
///
/// A weapon file is `<key>.weapon.ron`; Bevy's `file_stem()` yields `<key>.weapon`,
/// so the KEY (the [`WeaponName`] a [`GangerSpawn`](gdtf_battle_sim::situation::GangerSpawn)
/// references) is that stem minus a trailing `.weapon`. A stem without the infix is
/// returned unchanged (defensive — keeps a mis-named file's key its plain stem).
fn weapon_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".weapon").unwrap_or(stem).to_owned()
}
