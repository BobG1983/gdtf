//! GTW-257: builds the name-keyed [`WeaponRegistry`] from the loaded `assets/content/weapons/`
//! folder, plus the GTW-374 LIVE hot-reload that rebuilds it on a `*.weapon.ron` edit.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};

use crate::states::load::resources::{ActiveWeaponsFolderHandle, LoadHandles};

/// GTW-257: builds the name-keyed [`WeaponRegistry`] from the loaded
/// `assets/content/weapons/` folder, mirroring the fonts folder-load gating + the situation
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
///   STEM with the dedicated `.weapon` infix stripped (so `stub_pistol.weapon.ron` keys
///   `stub_pistol` — the weapon KEY), and inserts every `(WeaponName, WeaponSpec)` into
///   the registry. If ANY member spec is not yet in the collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries
///   next frame — so a partial / empty registry is never published while the folder
///   is non-empty. The registry holds the specs BY VALUE, so they survive the folder
///   handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_weapons(
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
        let Some(registry) =
            build_weapon_registry(asset_server, folders, weapon_specs, &handles.weapons)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) — retry
            // next frame (the system stays alive while the WeaponRegistry is absent).
            return;
        };

        // Insert the built registry — like GdtfTheme / CombatTuning it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the battle reads it.
        commands.insert_resource(registry);
        // GTW-374: insert the PERSISTENT folder handle alongside the registry — it
        // survives OnExit(Load) so the live hot-reload handler can re-enumerate the
        // folder's member handles to rebuild the registry on a `*.weapon.ron` edit, and
        // holding it keeps every member weapon asset loaded for the file-watcher.
        commands.insert_resource(ActiveWeaponsFolderHandle::new((*handles.weapons).clone()));
    }
}

/// Build the name-keyed [`WeaponRegistry`] from a loaded `weapons/` [`LoadedFolder`],
/// or [`None`] if the folder (or any member spec) is not yet in its collection.
///
/// Shared by [`resolve_weapons`] (the one-time `Load`-state build) and the GTW-374
/// [`redrive_weapons_on_asset_event`] (the live rebuild on a hot edit), so both build
/// the registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<WeaponSpec>`, read its [`WeaponSpec`] out of the collection, and key it by
/// the asset path's file STEM with the dedicated `.weapon` infix stripped. Returns
/// [`None`] (do NOT publish a partial registry) if the folder or any member spec is not
/// yet in its collection — the caller retries next frame.
fn build_weapon_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    weapon_specs: &Assets<RonAsset<WeaponSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<WeaponRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = WeaponRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<WeaponSpec> and read its spec
        // out of the collection.
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<WeaponSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: a member spec is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let spec = weapon_specs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.weapon` infix
        // stripped: `stub_pistol.weapon.ron`'s `file_stem()` is `stub_pistol.weapon`,
        // whose weapon KEY is `stub_pistol`. A handle with no resolvable path / stem is
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
    Some(registry)
}

/// `Update`: rebuild the [`WeaponRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/weapons/*.weapon.ron` — the GTW-374 LIVE weapon hot-reload, modelled on the
/// presenter's `redrive_fx_tuning_on_asset_event`.
///
/// A folder load fans out into one `RonAsset<WeaponSpec>` asset PER file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset
/// (not the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<WeaponSpec>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActiveWeaponsFolderHandle`]'s member handles via
/// [`build_weapon_registry`] — the SAME builder the one-time resolve uses, so a live
/// edit yields the same registry a restart would. Overwriting via [`ResMut`] marks the
/// registry changed, so the next battle setup resolves against the edited specs WITHOUT
/// a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`WeaponRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional
/// [`AssetServer`] / folder handle / `Assets` / [`WeaponRegistry`] borrows.
pub(in crate::states::load) fn redrive_weapons_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<WeaponSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveWeaponsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    weapon_specs: Option<Res<Assets<RonAsset<WeaponSpec>>>>,
    registry: Option<ResMut<WeaponRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(weapon_specs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, weapon_specs, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified weapon member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_weapon_registry(&asset_server, &folders, &weapon_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "weapon hot-reload: rebuilt WeaponRegistry from `assets/content/weapons/` ({} weapons)",
        registry.len(),
    );
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

#[cfg(test)]
mod test {
    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
        ecs::system::RunSystemOnce,
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};
    use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};

    use super::redrive_weapons_on_asset_event;
    use crate::states::load::{
        resources::ActiveWeaponsFolderHandle,
        systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A stub_pistol-shaped `WeaponSpec` with the given `damage` — parsed from inline
    /// RON so the test does not hand-assemble the `FireMode` `Vec`. Returns `None`
    /// (assert-fail) on a parse error rather than a denied `unwrap`.
    fn weapon_spec(damage: i32) -> Option<WeaponSpec> {
        let ron = format!(
            "(base_spread: 0.10, accuracy: 1.0, kickback: 0.05, fatal_bias: 0.0, \
             damage: {damage}, punch: 2, shred: 1, damage_type: Kinetic, \
             magazine: (size: 12, reload_tu: 12), \
             fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1)], \
             stable: false, handedness: OneHanded)",
        );
        let parsed = ron::de::from_str::<WeaponSpec>(&ron);
        assert!(
            parsed.is_ok(),
            "weapon fixture must parse: {:?}",
            parsed.as_ref().err()
        );
        parsed.ok()
    }

    /// A headless app with the real hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
    /// (registers `Assets<RonAsset<WeaponSpec>>`, `Assets<LoadedFolder>`, and the
    /// `AssetEvent` message buffers), the `WeaponSpec` RON loader, and the redrive
    /// system in `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"])
            .add_systems(Update, redrive_weapons_on_asset_event);
        app
    }

    /// Register a member weapon asset at `path` (so `AssetServer::get_path` resolves its
    /// stem) carrying `spec`, and return its typed handle. `load(path)` registers the
    /// path→id map synchronously; `insert` then provides the in-memory spec (the async
    /// load finds no file under the test CWD, so it never clobbers this).
    fn add_member(
        app: &mut App,
        path: &'static str,
        spec: WeaponSpec,
    ) -> Handle<RonAsset<WeaponSpec>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<WeaponSpec>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<WeaponSpec>>>()
            .insert(handle.id(), RonAsset::new(spec));
        assert!(inserted.is_ok(), "member spec insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(app: &mut App, members: &[Handle<RonAsset<WeaponSpec>>]) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// B2: a `Modified` for a member `*.weapon.ron` REBUILDS the `WeaponRegistry` from
    /// the folder's members — keyed by file stem — reflecting the edited spec.
    ///
    /// Pin-discriminating: dropping the rebuild leaves the OLD damage; mis-keying drops
    /// the `stub_pistol` entry.
    #[test]
    fn modified_member_rebuilds_weapon_registry() {
        let mut app = app();
        let Some(original) = weapon_spec(12) else {
            return;
        };
        let member = add_member(&mut app, "content/weapons/stub_pistol.weapon.ron", original);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveWeaponsFolderHandle::new(folder));
        // A stale baseline registry (empty) the rebuild must overwrite.
        app.world_mut().insert_resource(WeaponRegistry::default());
        app.update();

        // Hot-edit the member spec to a DISTINCT damage, fire Modified, rebuild.
        let Some(edited) = weapon_spec(6) else { return };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<WeaponSpec>>>()
            .get_mut(&member)
        {
            **asset = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let key = WeaponName::new("stub_pistol".to_owned());
        let damage = app
            .world()
            .get_resource::<WeaponRegistry>()
            .and_then(|r| r.spec(&key).map(|s| *s.damage));
        assert_eq!(
            damage,
            Some(6),
            "the hot-reload must rebuild the registry, keyed by stem, with the edited damage",
        );
    }

    /// B2: a hot-reload of a weapon member fires the Part C `info!` line naming what
    /// reloaded. Run via `run_system_once` on the calling thread so the thread-local
    /// `tracing` capture sees the emission (the schedule executor may run on a worker
    /// thread the capture would miss).
    ///
    /// Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn weapon_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(spec) = weapon_spec(12) else { return };
        let member = add_member(&mut app, "content/weapons/stub_pistol.weapon.ron", spec);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveWeaponsFolderHandle::new(folder));
        app.world_mut().insert_resource(WeaponRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_weapons_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("weapon hot-reload") && line.contains("WeaponRegistry")),
            "the weapon hot-reload must emit an info! line naming what reloaded; captured: {captured:?}",
        );
    }
}
