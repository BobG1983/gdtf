//! GTW-415: builds the name-keyed [`GangRegistry`] from the loaded `assets/content/gangs/`
//! folder, plus the LIVE hot-reload that rebuilds it on a `*.gang.ron` edit.
//!
//! The gang mirror of the GTW-257 weapons resolve
//! ([`resolve_weapons`](super::weapons::resolve_weapons)): the `Load` flow preloads the
//! `assets/content/gangs/` folder, this resolves it into the
//! [`GangRegistry`](gdtf_battle_sim::ganger::GangRegistry) the v2
//! [`setup_battle`](gdtf_battle_sim::situation::setup_battle) resolves every placed
//! ganger's `(gang, member)` ref against (GTW-414). Without this, the registry is never
//! populated from the shipped gang files and a real battle fails closed with
//! [`GangNotFound`](gdtf_battle_sim::situation::BattleSetupError).

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};

use crate::states::load::resources::{ActiveGangsFolderHandle, LoadHandles};

/// GTW-415: builds the name-keyed [`GangRegistry`] from the loaded
/// `assets/content/gangs/` folder, mirroring the GTW-257 weapons resolve shape exactly
/// (the gang mirror of [`resolve_weapons`](super::weapons::resolve_weapons)).
///
/// Called only while no [`GangRegistry`] resource exists yet (the caller's own-absence
/// guard), independently of the theme / tuning / weapons / armor / situation branches:
///
/// - Gates on the gangs folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every gang `.ron`
///   IN the folder is loaded — the weapons-folder pattern). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`GangRegistry`] so `Load` always exits with one present and never hangs on a
///   bad folder (the ADR-0003 error-path safety-net; a battle then fails closed with
///   [`GangNotFound`](gdtf_battle_sim::situation::BattleSetupError) rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<GangRoster>`, reads its [`GangRoster`] out of the
///   `Assets<RonAsset<GangRoster>>` collection, keys it by the asset path's file
///   STEM with the dedicated `.gang` infix stripped (so `gang_0.gang.ron` keys
///   `gang_0` — the gang KEY), and inserts every `(GangName, GangRoster)` into
///   the registry. If ANY member roster is not yet in the collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries
///   next frame — so a partial / empty registry is never published while the folder
///   is non-empty. The registry holds the rosters BY VALUE, so they survive the folder
///   handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_gangs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    gang_rosters: &Assets<RonAsset<GangRoster>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.gangs);

    // Failure path: a bad/missing gangs folder must not hang the app. Warn and insert an
    // EMPTY registry so Load always exits with one present (a battle then fails closed
    // with GangNotFound rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `gangs` folder failed to load; inserting an empty GangRegistry \
             (battles will fail closed on a missing gang key)",
        );
        commands.insert_resource(GangRegistry::default());
        return;
    }

    // Success path: once every gang file in the folder is loaded, read the LoadedFolder's
    // member handles and build the name-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_gang_registry(asset_server, folders, gang_rosters, &handles.gangs)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member roster) — retry
            // next frame (the system stays alive while the GangRegistry is absent).
            return;
        };

        // Insert the built registry — like the WeaponRegistry it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the battle reads it.
        commands.insert_resource(registry);
        // GTW-415: insert the PERSISTENT folder handle alongside the registry — it
        // survives OnExit(Load) so the live hot-reload handler can re-enumerate the
        // folder's member handles to rebuild the registry on a `*.gang.ron` edit, and
        // holding it keeps every member gang asset loaded for the file-watcher.
        commands.insert_resource(ActiveGangsFolderHandle::new((*handles.gangs).clone()));
    }
}

/// Build the name-keyed [`GangRegistry`] from a loaded `gangs/` [`LoadedFolder`], or
/// [`None`] if the folder (or any member roster) is not yet in its collection — the gang
/// mirror of `build_weapon_registry` (in the sibling `weapons` module).
///
/// Shared by [`resolve_gangs`] (the one-time `Load`-state build) and the GTW-415
/// [`redrive_gangs_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<GangRoster>`, read its [`GangRoster`] out of the collection, and key it by
/// the asset path's file STEM with the dedicated `.gang` infix stripped. Returns
/// [`None`] (do NOT publish a partial registry) if the folder or any member roster is not
/// yet in its collection — the caller retries next frame.
fn build_gang_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    gang_rosters: &Assets<RonAsset<GangRoster>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<GangRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = GangRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<GangRoster> and read its roster
        // out of the collection.
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<GangRoster>>();
        // One-frame loaded-but-not-yet-in-collection race: a member roster is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let roster = gang_rosters.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.gang` infix stripped:
        // `gang_0.gang.ron`'s `file_stem()` is `gang_0.gang`, whose gang KEY is `gang_0`.
        // A handle with no resolvable path / stem is skipped defensively (it would carry
        // no usable key).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| gang_key_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        registry.insert(GangName::new(stem), (**roster).clone());
    }
    Some(registry)
}

/// `Update`: rebuild the [`GangRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/gangs/*.gang.ron` — the GTW-415 LIVE gang hot-reload, the gang mirror of
/// `redrive_weapons_on_asset_event` (in the sibling `weapons` module).
///
/// A folder load fans out into one `RonAsset<GangRoster>` asset PER file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset
/// (not the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<GangRoster>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActiveGangsFolderHandle`]'s member handles via [`build_gang_registry`]
/// — the SAME builder the one-time resolve uses. Overwriting via [`ResMut`] marks the
/// registry changed, so the next battle setup resolves against the edited rosters WITHOUT
/// a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`GangRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional
/// [`AssetServer`] / folder handle / `Assets` / [`GangRegistry`] borrows.
pub(in crate::states::load) fn redrive_gangs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<GangRoster>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveGangsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    gang_rosters: Option<Res<Assets<RonAsset<GangRoster>>>>,
    registry: Option<ResMut<GangRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(gang_rosters),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, gang_rosters, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified gang member — a single rebuild from the latest in-memory
    // rosters covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) = build_gang_registry(&asset_server, &folders, &gang_rosters, &folder_handle)
    else {
        // A member roster is mid-reload (not yet back in the collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "gang hot-reload: rebuilt GangRegistry from `assets/content/gangs/` ({} gangs)",
        registry.len(),
    );
}

/// The gang KEY for a loaded gang file's stem — the stem with the dedicated `.gang`
/// infix stripped (GTW-415).
///
/// A gang file is `<key>.gang.ron`; Bevy's `file_stem()` yields `<key>.gang`, so the KEY
/// (the [`GangName`] a [`PlacedGanger`](gdtf_battle_sim::situation::PlacedGanger)
/// references) is that stem minus a trailing `.gang`. A stem without the infix is returned
/// unchanged (defensive — keeps a mis-named file's key its plain stem).
fn gang_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".gang").unwrap_or(stem).to_owned()
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
    use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};

    use super::redrive_gangs_on_asset_event;
    use crate::states::load::{
        resources::ActiveGangsFolderHandle, systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A one-member `GangRoster` whose lone member's name is `member_name` — parsed from
    /// inline RON so the test does not hand-assemble the member record. Returns `None`
    /// (assert-fail) on a parse error rather than a denied `unwrap`.
    fn gang_roster(member_name: &str) -> Option<GangRoster> {
        let ron = format!(
            "(members: [(name: \"{member_name}\", speed: 3.0, aim: 3.0, strength: 4.0, \
             toughness: 12.0, reflexes: 3.0, cool: 6.0, grit: 19.0, luck: 1.0, \
             armor: \"flak_vest\", weapon: \"stub_pistol\")])",
        );
        let parsed = ron::de::from_str::<GangRoster>(&ron);
        assert!(
            parsed.is_ok(),
            "gang fixture must parse: {:?}",
            parsed.as_ref().err()
        );
        parsed.ok()
    }

    /// A headless app with the real hot-reload wiring (the gang mirror of the weapons
    /// test app).
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<GangRoster>(vec!["gang.ron"])
            .add_systems(Update, redrive_gangs_on_asset_event);
        app
    }

    /// Register a member gang asset at `path` carrying `roster`, returning its handle —
    /// the gang mirror of the weapons test's `add_member`.
    fn add_member(
        app: &mut App,
        path: &'static str,
        roster: GangRoster,
    ) -> Handle<RonAsset<GangRoster>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<GangRoster>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<GangRoster>>>()
            .insert(handle.id(), RonAsset::new(roster));
        assert!(inserted.is_ok(), "member roster insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(app: &mut App, members: &[Handle<RonAsset<GangRoster>>]) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// A `Modified` for a member `*.gang.ron` REBUILDS the `GangRegistry` from the
    /// folder's members — keyed by file stem — reflecting the edited roster.
    ///
    /// Pin-discriminating: dropping the rebuild leaves the OLD member name; mis-keying
    /// drops the `gang_0` entry.
    #[test]
    fn modified_member_rebuilds_gang_registry() {
        let mut app = app();
        let Some(original) = gang_roster("Alex Mercer") else {
            return;
        };
        let member = add_member(&mut app, "content/gangs/gang_0.gang.ron", original);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveGangsFolderHandle::new(folder));
        // A stale baseline registry (empty) the rebuild must overwrite.
        app.world_mut().insert_resource(GangRegistry::default());
        app.update();

        // Hot-edit the member roster to a DISTINCT member name, fire Modified, rebuild.
        let Some(edited) = gang_roster("Renamed Merc") else {
            return;
        };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<GangRoster>>>()
            .get_mut(&member)
        {
            **asset = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let key = GangName::new("gang_0".to_owned());
        let first_member_name = app
            .world()
            .get_resource::<GangRegistry>()
            .and_then(|r| r.roster(&key))
            .and_then(|roster| roster.members.first())
            .map(|m| (*m.name).clone());
        assert_eq!(
            first_member_name,
            Some("Renamed Merc".to_owned()),
            "the hot-reload must rebuild the registry, keyed by stem, with the edited roster",
        );
    }

    /// A hot-reload of a gang member fires the `info!` line naming what reloaded. Run via
    /// `run_system_once` on the calling thread so the thread-local `tracing` capture sees
    /// the emission (the schedule executor may run on a worker thread the capture would
    /// miss).
    ///
    /// Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn gang_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(roster) = gang_roster("Alex Mercer") else {
            return;
        };
        let member = add_member(&mut app, "content/gangs/gang_0.gang.ron", roster);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveGangsFolderHandle::new(folder));
        app.world_mut().insert_resource(GangRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_gangs_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("gang hot-reload") && line.contains("GangRegistry")),
            "the gang hot-reload must emit an info! line naming what reloaded; captured: {captured:?}",
        );
    }
}
