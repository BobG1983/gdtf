//! GTW-505: builds the name-keyed [`MeleeWeaponRegistry`] from the loaded
//! `assets/content/weapons/melee/` folder, plus the LIVE hot-reload that rebuilds it on a
//! `*.melee_weapon.ron` edit — the melee mirror of the sibling
//! [`weapons`](super::weapons) (ranged) resolver.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName};

use crate::states::load::resources::{ActiveMeleeWeaponsFolderHandle, LoadHandles};

/// GTW-505: builds the name-keyed [`MeleeWeaponRegistry`] from the loaded
/// `assets/content/weapons/melee/` folder, the melee mirror of
/// [`resolve_weapons`](super::weapons::resolve_weapons).
///
/// Called only while no [`MeleeWeaponRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the other resolve branches:
///
/// - Gates on the melee-weapons folder's [`RecursiveDependencyLoadState`]`::Loaded`. On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`MeleeWeaponRegistry`] so `Load` always exits with one present (the ADR-0003
///   fail-safe; a battle then fails closed with `MeleeWeaponNotFound` rather than
///   crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<MeleeWeaponSpec>`, reads its [`MeleeWeaponSpec`] out of the collection, keys
///   it by the asset path's file STEM with the dedicated `.melee_weapon` infix stripped
///   (so `fists.melee_weapon.ron` keys `fists`), and inserts every `(WeaponName,
///   MeleeWeaponSpec)` into the registry. The one-frame loaded-but-not-yet-in-collection
///   race retries next frame (it never publishes a partial registry).
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_melee_weapons(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    melee_specs: &Assets<RonAsset<MeleeWeaponSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.melee_weapons);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `weapons/melee` folder failed to load; inserting an empty \
             MeleeWeaponRegistry (battles will fail closed on a missing melee weapon key)",
        );
        commands.insert_resource(MeleeWeaponRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_melee_weapon_registry(asset_server, folders, melee_specs, &handles.melee_weapons)
        else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays alive
            // while the MeleeWeaponRegistry is absent).
            return;
        };

        // Persist past OnExit(Load) (NOT removed in cleanup) — the battle reads it.
        commands.insert_resource(registry);
        // GTW-505: insert the PERSISTENT folder handle alongside the registry so the live
        // hot-reload handler can re-enumerate the folder's member handles, and holding it
        // keeps every member melee weapon asset loaded for the file-watcher.
        commands.insert_resource(ActiveMeleeWeaponsFolderHandle::new(
            (*handles.melee_weapons).clone(),
        ));
    }
}

/// Build the name-keyed [`MeleeWeaponRegistry`] from a loaded `weapons/melee/`
/// [`LoadedFolder`], or [`None`] if the folder (or any member spec) is not yet in its
/// collection.
///
/// Shared by [`resolve_melee_weapons`] (the one-time `Load`-state build) and
/// [`redrive_melee_weapons_on_asset_event`] (the live rebuild), so both build the registry
/// IDENTICALLY — the [`build_weapon_registry`](super::weapons) (ranged) mirror.
fn build_melee_weapon_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    melee_specs: &Assets<RonAsset<MeleeWeaponSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<MeleeWeaponRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = MeleeWeaponRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<MeleeWeaponSpec> and read its spec.
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<MeleeWeaponSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: bail (do NOT build a partial
        // registry) so the caller re-polls.
        let spec = melee_specs.get(&handle)?;
        // Key by the file STEM with the dedicated `.melee_weapon` infix stripped:
        // `fists.melee_weapon.ron`'s `file_stem()` is `fists.melee_weapon`, whose KEY is
        // `fists`. A handle with no resolvable path / stem is skipped defensively.
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| melee_weapon_key_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        registry.insert(WeaponName::new(stem), (**spec).clone());
    }
    Some(registry)
}

/// `Update`: rebuild the [`MeleeWeaponRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/weapons/melee/*.melee_weapon.ron` — the LIVE melee-weapon hot-reload,
/// the [`redrive_weapons_on_asset_event`](super::weapons::redrive_weapons_on_asset_event)
/// mirror.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`MeleeWeaponRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1). Param-only (`bevy-traps.md` #7).
pub(in crate::states::load) fn redrive_melee_weapons_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<MeleeWeaponSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveMeleeWeaponsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    melee_specs: Option<Res<Assets<RonAsset<MeleeWeaponSpec>>>>,
    registry: Option<ResMut<MeleeWeaponRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(melee_specs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, melee_specs, registry)
    else {
        // Drain so a pre-resolve event does not linger; nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified melee weapon member — a single rebuild from the latest
    // in-memory specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_melee_weapon_registry(&asset_server, &folders, &melee_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the existing
        // registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "melee weapon hot-reload: rebuilt MeleeWeaponRegistry from \
         `assets/content/weapons/melee/` ({} melee weapons)",
        registry.len(),
    );
}

/// The melee-weapon KEY for a loaded melee weapon file's stem — the stem with the
/// dedicated `.melee_weapon` infix stripped (GTW-505).
///
/// A melee weapon file is `<key>.melee_weapon.ron`; Bevy's `file_stem()` yields
/// `<key>.melee_weapon`, so the KEY is that stem minus a trailing `.melee_weapon`. A stem
/// without the infix is returned unchanged (defensive).
fn melee_weapon_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".melee_weapon")
        .unwrap_or(stem)
        .to_owned()
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
    use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName};

    use super::redrive_melee_weapons_on_asset_event;
    use crate::states::load::{
        resources::ActiveMeleeWeaponsFolderHandle,
        systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A fists-shaped `MeleeWeaponSpec` with the given `damage` — parsed from inline RON so
    /// the test does not hand-assemble the `FightMode` `Vec`. Returns `None` (assert-fail)
    /// on a parse error rather than a denied `unwrap`. Proves the melee spec parses (C6).
    fn melee_spec(damage: i32) -> Option<MeleeWeaponSpec> {
        let ron = format!(
            "(damage: {damage}, punch: 0, shred: 0, damage_type: Kinetic, fatal_bias: 0.0, \
             handedness: OneHanded, reach: 1, \
             fight_mode: [(kind: Swing, tu_cost: 20, strikes: 1)])",
        );
        let parsed = ron::de::from_str::<MeleeWeaponSpec>(&ron);
        assert!(
            parsed.is_ok(),
            "melee weapon fixture must parse: {:?}",
            parsed.as_ref().err()
        );
        parsed.ok()
    }

    /// A headless app with the real melee hot-reload wiring.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<MeleeWeaponSpec>(vec!["melee_weapon.ron"])
            .add_systems(Update, redrive_melee_weapons_on_asset_event);
        app
    }

    /// Register a member melee weapon asset at `path` carrying `spec`, returning its typed
    /// handle (the ranged `add_member` mirror).
    fn add_member(
        app: &mut App,
        path: &'static str,
        spec: MeleeWeaponSpec,
    ) -> Handle<RonAsset<MeleeWeaponSpec>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<MeleeWeaponSpec>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<MeleeWeaponSpec>>>()
            .insert(handle.id(), RonAsset::new(spec));
        assert!(inserted.is_ok(), "member spec insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(
        app: &mut App,
        members: &[Handle<RonAsset<MeleeWeaponSpec>>],
    ) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// C6: a `Modified` for a member `*.melee_weapon.ron` REBUILDS the
    /// `MeleeWeaponRegistry` from the folder's members — keyed by file stem (minus the
    /// `.melee_weapon` infix) — reflecting the edited spec. Pin-discriminating: dropping
    /// the rebuild leaves the OLD damage; mis-keying drops the `fists` entry.
    #[test]
    fn modified_member_rebuilds_melee_weapon_registry() {
        let mut app = app();
        let Some(original) = melee_spec(4) else {
            return;
        };
        let member = add_member(
            &mut app,
            "content/weapons/melee/fists.melee_weapon.ron",
            original,
        );
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveMeleeWeaponsFolderHandle::new(folder));
        app.world_mut()
            .insert_resource(MeleeWeaponRegistry::default());
        app.update();

        let Some(edited) = melee_spec(7) else { return };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<MeleeWeaponSpec>>>()
            .get_mut(&member)
        {
            **asset = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let key = WeaponName::new("fists".to_owned());
        let damage = app
            .world()
            .get_resource::<MeleeWeaponRegistry>()
            .and_then(|r| r.spec(&key).map(|s| *s.damage));
        assert_eq!(
            damage,
            Some(7),
            "the hot-reload must rebuild the registry, keyed by stem, with the edited damage",
        );
    }

    /// C6: a hot-reload of a melee weapon member fires the `info!` line naming what
    /// reloaded. Run via `run_system_once` so the thread-local `tracing` capture sees the
    /// emission. Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn melee_weapon_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(spec) = melee_spec(4) else { return };
        let member = add_member(
            &mut app,
            "content/weapons/melee/fists.melee_weapon.ron",
            spec,
        );
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveMeleeWeaponsFolderHandle::new(folder));
        app.world_mut()
            .insert_resource(MeleeWeaponRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_melee_weapons_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("melee weapon hot-reload")
                    && line.contains("MeleeWeaponRegistry")),
            "the melee weapon hot-reload must emit an info! line naming what reloaded; \
             captured: {captured:?}",
        );
    }
}
