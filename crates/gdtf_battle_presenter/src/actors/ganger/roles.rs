//! The DATA-DRIVEN per-faction base-actor table and its RON load/resolve chain.

use bevy::{asset::AssetEvent, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::Faction;
use serde::Deserialize;

use crate::TileIndex;

/// The DATA-DRIVEN per-faction base actor table — each faction (gang) -> its base
/// [`TileIndex`] into the character sheet.
///
/// Loaded from the loose `assets/sprites/character_roles.spritedef.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`CharacterRoles`] resource before battle time (see [`resolve_character_roles`]),
/// exactly mirroring the S4 `tile_roles.ron` / `load_tile_roles` / `resolve_tile_roles`
/// chain. Each faction's actor is a contiguous run of 4 cells in the sheet; the drawn
/// index is `base + facing_frame` ([`facing_frame`](super::facing_frame)). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the FACTIONS.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<CharacterRoles>`] requires of
/// its payload). At minimum faction 0 and faction 1 map to two visibly distinct actor
/// base tiles; a faction with no authored base falls back to [`Self::faction_0`] (the
/// only design factions today are 0 and 1, the two gangs).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct CharacterRoles {
    /// Faction (gang) 0's base actor tile index — the first gang's 4-frame actor.
    pub faction_0: TileIndex,
    /// Faction (gang) 1's base actor tile index — the opposing gang's 4-frame actor,
    /// authored to a visibly distinct actor from [`Self::faction_0`].
    pub faction_1: TileIndex,
}

impl CharacterRoles {
    /// The base actor [`TileIndex`] for `faction`, read from this resource.
    ///
    /// Faction 0 -> [`Self::faction_0`], faction 1 -> [`Self::faction_1`]. Any other
    /// gang index (none exist in the current two-gang design) falls back to
    /// [`Self::faction_0`] rather than panicking on an out-of-table faction.
    ///
    /// Not `const`: reading the gang index derefs [`Faction`]'s derived [`Deref`], which
    /// is not a `const` impl (the `f32`-newtype / `Deref`-blocks-const idiom).
    #[must_use]
    pub fn base_for(&self, faction: Faction) -> TileIndex {
        match *faction {
            1 => self.faction_1,
            _ => self.faction_0,
        }
    }
}

/// The path of the loose character-role RON, relative to the asset source root.
const CHARACTER_ROLES_RON_PATH: &str = "sprites/character_roles.spritedef.ron";

/// The in-flight handle to the character-role RON, held until it resolves into
/// [`CharacterRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no
/// domain meaning; this name says "the character-role table being loaded"). Inserted by
/// [`load_character_roles`] and read by [`resolve_character_roles`] — the S4
/// `TileRolesHandle` precedent for the character table.
#[derive(Resource, Deref, Debug, Clone)]
pub struct CharacterRolesHandle(Handle<RonAsset<CharacterRoles>>);

impl CharacterRolesHandle {
    /// Wrap the in-flight character-role RON handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<CharacterRoles>>) -> Self {
        Self(handle)
    }
}

/// `Startup`: kick off the `character_roles.ron` load, storing its typed handle.
///
/// Loads `sprites/character_roles.spritedef.ron` as a
/// [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) through the generic GTW-136 loader
/// and inserts the [`CharacterRolesHandle`] the [`resolve_character_roles`] poll system
/// reads — the S4 [`load_tile_roles`](crate::load_tile_roles) precedent. Takes
/// `Option<Res<AssetServer>>` so a `MinimalPlugins` headless app with no [`AssetServer`]
/// no-ops rather than panicking (`bevy-traps.md` #1); under `DefaultPlugins` (the app +
/// the `AssetServer` harness) the load fires for real and the resolve runs.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_character_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<CharacterRoles>>(CHARACTER_ROLES_RON_PATH);
    commands.insert_resource(CharacterRolesHandle::new(handle));
}

/// `Update` (gated until [`CharacterRoles`] is resolved): resolve the loaded RON into
/// the presenter-owned [`CharacterRoles`] resource.
///
/// Once the [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<CharacterRoles>>` (a transient one-frame "loaded but not yet in the
/// collection" state simply leaves it un-inserted this pass — retried next frame), it
/// clones the deserialized [`CharacterRoles`] out and inserts it as the resident
/// resource so it is present before the first ganger draws. Run only while
/// [`CharacterRolesHandle`] exists AND [`CharacterRoles`] does NOT (the plugin's
/// run-condition), so it inserts once — the S4 [`resolve_tile_roles`](crate::resolve_tile_roles)
/// precedent.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert,
/// [`Res<CharacterRolesHandle>`] for the handle,
/// [`Res<Assets<RonAsset<CharacterRoles>>>`] for the loaded asset.
pub fn resolve_character_roles(
    mut commands: Commands,
    handle: Res<CharacterRolesHandle>,
    roles_assets: Res<Assets<RonAsset<CharacterRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until CharacterRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}

/// `Update` (unguarded; self-gates on its [`Option`] borrows): live-reload the resident
/// [`CharacterRoles`] resource when its `character_roles.ron` is re-saved (GTW-375 C4).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`CharacterRoles`]`>>` — asset
/// events are MESSAGES in Bevy 0.19, so this is a `MessageReader`, not an `EventReader`
/// (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the in-flight
/// [`CharacterRolesHandle`]; events for any other handle are ignored. On a match it clones
/// the refreshed [`CharacterRoles`] out of the `Assets` collection and overwrites the
/// resident resource through [`ResMut`] — which MARKS it changed, so
/// [`reindex_ganger_sprites_on_character_roles_change`] re-indexes every mapped ganger
/// sprite against the new atlas indices the very next frame. Mirrors
/// [`redrive_tile_roles_on_asset_event`](crate::redrive_tile_roles_on_asset_event).
///
/// [`CharacterRoles`] is NOT `Copy` (it derives `Clone`), so the overwrite is
/// `(**updated).clone()`.
///
/// Guarded so it never panics before the load chain has run (pre-resolve): it takes the
/// handle / the `Assets` collection / the [`CharacterRoles`] resource as [`Option`]al
/// borrows, draining the reader and returning early if any is missing (`bevy-traps.md` #1)
/// so a pre-resolve event does not linger and re-fire later.
///
/// [`reindex_ganger_sprites_on_character_roles_change`]: super::systems::reindex_ganger_sprites_on_character_roles_change
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle / `Assets` /
/// [`CharacterRoles`] borrows.
pub fn redrive_character_roles_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<CharacterRoles>>>,
    handle: Option<Res<CharacterRolesHandle>>,
    roles_assets: Option<Res<Assets<RonAsset<CharacterRoles>>>>,
    roles: Option<ResMut<CharacterRoles>>,
) {
    let (Some(handle), Some(roles_assets), Some(mut roles)) = (handle, roles_assets, roles) else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to re-resolve yet.
        events.clear();
        return;
    };

    let active_id = handle.id();
    // Act once per frame even if several Modified events arrive: a single re-resolve from
    // the latest in-memory value covers them all.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { id } if *id == active_id));
    if !modified {
        return;
    }

    let Some(updated) = roles_assets.get(&**handle) else {
        // Modified but not currently in the collection (a transient reload state) — leave
        // the existing roles until it settles; the next event re-fires.
        return;
    };
    // CharacterRoles derives Clone (NOT Copy), so clone the refreshed table out of the asset.
    *roles = (**updated).clone();
    // GTW-374 Part C convention: log EVERY hot-reload path naming what reloaded, so a live
    // character-role edit can be traced (mirrors the tile / FX / combat / theme handlers).
    info!(
        "character hot-reload: re-resolved CharacterRoles from `sprites/character_roles.spritedef.ron`"
    );
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, Assets, Handle},
        ecs::system::RunSystemOnce,
        log::{
            tracing::{
                Event, Subscriber,
                field::{Field, Visit},
                subscriber::with_default,
            },
            tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
        },
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};

    use super::{CharacterRoles, CharacterRolesHandle, redrive_character_roles_on_asset_event};
    use crate::TileIndex;

    /// A scoped `tracing` layer that records each event's `message` field — the minimal
    /// capture needed to prove the `info!` hot-reload line fired. Mirrors the GTW-374
    /// combat redrive test's / `terrain/roles.rs`'s `CaptureLayer` (no shared util is
    /// reachable from this crate).
    struct CaptureLayer {
        /// The shared buffer captured messages append to.
        messages: Arc<Mutex<Vec<String>>>,
    }

    /// Pulls the `message` field's debug rendering out of a `tracing` event.
    struct MessageVisitor {
        /// The captured message text, if a `message` field was visited.
        message: Option<String>,
    }

    impl Visit for MessageVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = Some(format!("{value:?}"));
            }
        }
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = MessageVisitor { message: None };
            event.record(&mut visitor);
            if let Some(message) = visitor.message
                && let Ok(mut buffer) = self.messages.lock()
            {
                buffer.push(message);
            }
        }
    }

    /// Run `body` with a scoped [`CaptureLayer`] active, returning every captured message.
    ///
    /// The subscriber is scoped to this call (`with_default`), so it never leaks into other
    /// tests; the returned `Vec` is in emission order.
    fn capture_logs(body: impl FnOnce()) -> Vec<String> {
        let messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let layer = CaptureLayer {
            messages: Arc::clone(&messages),
        };
        let subscriber = Registry::default().with(layer);
        with_default(subscriber, body);
        messages
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default()
    }

    /// A headless app with the real character-role hot-reload wiring: `MinimalPlugins` +
    /// `AssetPlugin` (so `Assets<RonAsset<CharacterRoles>>` and the `AssetEvent` message
    /// buffer exist), the `CharacterRoles` RON loader registered, and the redrive system in
    /// `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<CharacterRoles>()
            .add_systems(Update, redrive_character_roles_on_asset_event);
        app
    }

    /// A `CharacterRoles` whose faction-0 base is `faction_0` and faction-1 base is `9` —
    /// the distinct fixtures the re-resolve test contrasts.
    fn roles_with_faction_0(faction_0: usize) -> CharacterRoles {
        CharacterRoles {
            faction_0: TileIndex::new(faction_0),
            faction_1: TileIndex::new(9),
        }
    }

    /// Add a `RonAsset<CharacterRoles>` to the collection and return its handle.
    fn add_asset(app: &mut App, roles: CharacterRoles) -> Handle<RonAsset<CharacterRoles>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<CharacterRoles>>>()
            .add(RonAsset::new(roles))
    }

    /// Overwrite the in-memory payload of an already-added character-role asset (the hot
    /// edit the file-watcher would make on a `character_roles.ron` save).
    fn hot_edit(app: &mut App, handle: &Handle<RonAsset<CharacterRoles>>, roles: CharacterRoles) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<CharacterRoles>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            **asset = roles;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id (standing in for the
    /// file-watcher's reload signal).
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<CharacterRoles>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// C10(b): on a matching `Modified` for the active character-role handle, the redrive
    /// system re-resolves the resident `CharacterRoles` resource to the UPDATED in-memory
    /// payload — the live character hot-reload. Driven through the real registered system
    /// via `app.update()`.
    ///
    /// Pin-discriminating: dropping the re-resolve leaves `CharacterRoles` on the OLD
    /// faction-0 index; a wrong id filter would re-resolve on any handle's event.
    #[test]
    fn modified_event_reresolves_character_roles() {
        let mut app = app();

        let baseline = roles_with_faction_0(12);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(CharacterRolesHandle::new(handle.clone()));

        // First update: no event, the resource is untouched.
        app.update();

        // Hot-edit the asset to a DISTINCT table, then fire a Modified.
        let edited = roles_with_faction_0(200);
        assert_ne!(edited, baseline, "precondition: the edit must differ");
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();

        assert_eq!(
            app.world().get_resource::<CharacterRoles>(),
            Some(&edited),
            "a Modified for the active character-role handle must re-resolve CharacterRoles \
             to the new values",
        );
    }

    /// C10(b): a `Modified` for a DIFFERENT asset id leaves `CharacterRoles` untouched — the
    /// filter is on the ACTIVE handle id only.
    ///
    /// Pin-discriminating: dropping the id filter re-resolves on any character-role asset's
    /// event.
    #[test]
    fn modified_event_for_other_id_does_not_reresolve() {
        let mut app = app();

        let baseline = roles_with_faction_0(12);
        let active = add_asset(&mut app, baseline.clone());
        // A second, unrelated character-role asset whose value differs from the active one.
        let other = add_asset(&mut app, roles_with_faction_0(99));
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(CharacterRolesHandle::new(active));

        app.update();
        // Edit the OTHER asset, fire Modified for it only.
        hot_edit(&mut app, &other, roles_with_faction_0(33));
        inject_modified(&mut app, &other);
        app.update();

        assert_eq!(
            app.world().get_resource::<CharacterRoles>(),
            Some(&baseline),
            "a Modified for a non-active character-role id must NOT re-resolve CharacterRoles",
        );
    }

    /// C7: the hot-reload `info!` line FIRES on the real re-resolve path, naming what
    /// reloaded. The redrive system is run via `run_system_once` on the calling thread
    /// (inside the scoped `tracing` subscriber) so the capture — which is thread-local —
    /// sees the emission (GTW-374 thread-local capture lesson).
    ///
    /// Pin-discriminating: removing the `info!` from the redrive leaves the capture empty
    /// and this assert fails.
    #[test]
    fn hot_reload_logs_an_info_line() {
        let mut app = app();
        let baseline = roles_with_faction_0(12);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(CharacterRolesHandle::new(handle.clone()));
        // Stage the hot edit + the Modified message, then run the redrive synchronously
        // inside the capture scope.
        hot_edit(&mut app, &handle, roles_with_faction_0(200));
        inject_modified(&mut app, &handle);

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_character_roles_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured.iter().any(
                |line| line.contains("character hot-reload") && line.contains("CharacterRoles")
            ),
            "the character-role hot-reload must emit an info! line naming what reloaded; \
             captured: {captured:?}",
        );
    }
}
