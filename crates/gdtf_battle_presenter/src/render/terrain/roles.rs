//! The atlas-index newtype, the DATA-DRIVEN terrain tile-role table, and its RON
//! load/resolve chain.

use bevy::{asset::AssetEvent, prelude::*};
use gdtf_assets::RonAsset;
use serde::{Deserialize, Serialize};

/// An index into the terrain sheet's atlas layout — WHICH 16x16 tile a role draws.
///
/// A named newtype over `usize` (no-bare-types: an atlas index is a domain value, not
/// a bare `usize`), [`Deref`]ing to it so a consumer reads the index straight through.
/// `#[serde(transparent)]` so an authored `tile_roles.ron` field parses as a bare
/// integer (`floor: 6`), not a one-field struct. [`Serialize`] (with the same
/// transparency) lets the GTW-373 round-trip-identity test re-serialize a loaded table.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TileIndex(usize);

impl TileIndex {
    /// Build a tile index from its layout position.
    ///
    /// `usize` is the index space of the atlas layout's `textures` collection (a
    /// collection-index, the no-bare-types framework-plumbing carve-out for the inner
    /// value), wrapped here as the named domain [`TileIndex`].
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

impl TileRoles {
    /// Resolve a per-def terrain GRAPHIC-key string to its [`TileIndex`] in this table —
    /// the GTW-493 presenter seam (the sim spawns a
    /// [`TerrainGraphicKey`](gdtf_battle_sim::TerrainGraphicKey) on every terrain entity,
    /// keyed in THIS `TileRoles` vocabulary; the presenter resolves it here).
    ///
    /// The authored `graphic_name` follows the `tile_roles.ron` vocabulary
    /// (`"floor"` / `"wall"` / `"wall_ew"` / `"cover"` / `"slab"` / `"rubble"` / …), so this
    /// maps the key string onto the matching role field. Returns [`None`] for an unrecognized key
    /// — the caller then FALLS BACK to its presenter-owned `TileRole`-table default keyed on
    /// the cell's [`TerrainKind`](gdtf_battle_sim::TerrainKind), so an out-of-vocabulary
    /// def still draws (no panic) rather than vanishing.
    ///
    /// Two `Cover` defs whose `graphic_name`s differ (e.g. `"cover"` vs `"rubble"`)
    /// therefore resolve to DISTINCT indices through this method — the per-def graphic the
    /// ticket requires, which the role-table default (keyed only on the shared
    /// [`TerrainKind::Cover`](gdtf_battle_sim::TerrainKind)) cannot express.
    #[must_use]
    pub fn index_for_key(&self, key: &str) -> Option<TileIndex> {
        let index = match key {
            "floor" => self.floor,
            "floor_alt_panel" => self.floor_alt_panel,
            "wall" => self.wall,
            "wall_ew" => self.wall_ew,
            "cover" => self.cover,
            "slab" => self.slab,
            "rubble" => self.rubble,
            "slab_destroyed" => self.slab_destroyed,
            "door" => self.door,
            "stair_up" => self.stair_up,
            "stair_down" => self.stair_down,
            "ladder" => self.ladder,
            _ => return None,
        };
        Some(index)
    }
}

/// The DATA-DRIVEN terrain tile-role table — each terrain ROLE → its [`TileIndex`].
///
/// Loaded from the loose `assets/sprites/tile_roles.spritedef.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`TileRoles`] resource before battle time (see [`resolve_tile_roles`]). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the ROLES.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), [`Serialize`] (the GTW-373 round-trip-identity test re-emits a loaded
/// table), and [`TypePath`] (the bound [`RonAsset<TileRoles>`] requires of its payload).
/// The `floor_alt_*` / `door` fields are authored for future variety; the default S4 draw
/// uses `floor` / `wall` / `cover` / `slab` / `rubble`.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct TileRoles {
    /// The default walkable-ground tile — in-range cells with no wall / cover / slab.
    pub floor:           TileIndex,
    /// A bolted/riveted grey-panel floor alternate (future variety).
    pub floor_alt_panel: TileIndex,
    /// The default (NS-orientation) [`TerrainKind::Wall`](gdtf_battle_sim::TerrainKind::Wall)
    /// tile — solid fixed geometry, the north-south-running wall strip.
    pub wall:            TileIndex,
    /// The **east-west-running** wall tile (GTW-469) — the [`wall`](TileRoles::wall) NS sprite
    /// rotated 90° (its row-2 counterpart on the terrain sheet). A distinct AUTHORED orientation:
    /// an EW-wall [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) is `sim_kind = Wall`
    /// just like the NS one (LOS / movement blocking is IDENTICAL — orientation is
    /// presentation-only) but names `graphic_name = "wall_ew"`, which resolves here so the two
    /// orientations draw perpendicular sprites. The author places the correct orientation in a
    /// prefab; orientation is never presenter-inferred.
    pub wall_ew:         TileIndex,
    /// The [`TerrainKind::Cover`](gdtf_battle_sim::TerrainKind::Cover) tile — a chest-high cover prop.
    pub cover:           TileIndex,
    /// The [`SurfaceGrid`](gdtf_battle_sim::SurfaceGrid) `Present`-slab tile — a raised elevated deck.
    pub slab:            TileIndex,
    /// The destroyed-cover / damaged tile — broken debris scatter.
    pub rubble:          TileIndex,
    /// The destroyed-**slab** tile — the engineer's-choice treatment for a smashed
    /// floor/roof slab (GTW-367 C3, mirroring how [`rubble`](TileRoles::rubble) is the
    /// destroyed-**cover** tile).
    ///
    /// CHOICE + WHY: a destroyed slab is rendered as the same broken-debris-scatter tile
    /// the rest of the terrain set uses for destruction (sharing the `rubble` atlas index
    /// `295`). A collapsed elevated deck reads as a field of rubble/debris, which keeps the
    /// destruction vocabulary consistent (smashed cover → rubble; smashed slab → rubble),
    /// and — crucially — keeps the cell still reading as *terrain* (a swap, never a hole)
    /// so the in-place mutation (no despawn) preserves the sprite. It is authored as its
    /// OWN role field (not a `rubble` alias) precisely so art can later DIVERGE it without
    /// touching the cover path.
    ///
    /// ART-REVIEW: the destroyed-slab treatment is a placeholder reuse of the rubble tile.
    /// A collapsed floor/roof slab arguably wants a DISTINCT visual — a hole punched
    /// through to the level below, a cracked/shattered deck, or a scorch — rather than the
    /// generic ground-debris scatter. Flag for art to author a dedicated destroyed-slab
    /// tile and re-point this index in `assets/sprites/tile_roles.spritedef.ron`.
    pub slab_destroyed:  TileIndex,
    /// A doorway / hatch tile (authored for future variety).
    pub door:            TileIndex,
    /// The [`VerticalLink`](gdtf_battle_sim::VerticalLink) `Stair`-endpoint tile drawn
    /// where the active storey is the link's LOWER cell — i.e. you ASCEND from here
    /// (atlas index `29`). GTW-373 (user ruling 2026-06-23) SUPERSEDES the GTW-359 OQ-3
    /// single-`stair`-`77` constant: the stair role is split into [`stair_up`] /
    /// [`stair_down`], chosen by link direction relative to the active storey (see
    /// [`draw_vertical_links`](crate::draw_vertical_links)).
    ///
    /// [`stair_up`]: TileRoles::stair_up
    /// [`stair_down`]: TileRoles::stair_down
    pub stair_up:        TileIndex,
    /// The [`VerticalLink`](gdtf_battle_sim::VerticalLink) `Stair`-endpoint tile drawn
    /// where the active storey is the link's UPPER cell — i.e. you DESCEND from here
    /// (atlas index `28`). The down-facing companion of [`stair_up`]; same GTW-373 split
    /// of the former single `stair` role, chosen by link direction.
    ///
    /// [`stair_up`]: TileRoles::stair_up
    pub stair_down:      TileIndex,
    /// The [`VerticalLink`](gdtf_battle_sim::VerticalLink) `Ladder`-endpoint tile — a
    /// ladder cell (atlas index `235`, an UNCHANGED system constant per the user OQ-3
    /// ruling). Drawn at each ladder link cell on the active storey by the GTW-359
    /// link-cell draw (a ladder is drawn the same tile both up and down).
    pub ladder:          TileIndex,
}

/// The path of the loose tile-role RON, relative to the asset source root.
const TILE_ROLES_RON_PATH: &str = "sprites/tile_roles.spritedef.ron";

/// The in-flight handle to the tile-role RON, held until it resolves into [`TileRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no
/// domain meaning; this name says "the tile-role table being loaded"). Inserted by
/// [`load_tile_roles`] and read by [`resolve_tile_roles`].
#[derive(Resource, Deref, Debug, Clone)]
pub struct TileRolesHandle(Handle<RonAsset<TileRoles>>);

impl TileRolesHandle {
    /// Wrap the in-flight tile-role RON handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<TileRoles>>) -> Self {
        Self(handle)
    }
}

/// `Startup`: kick off the `tile_roles.ron` load, storing its typed handle.
///
/// Loads `sprites/tile_roles.spritedef.ron` as a [`RonAsset<TileRoles>`](gdtf_assets::RonAsset)
/// through the generic GTW-136 loader and inserts the [`TileRolesHandle`] the
/// [`resolve_tile_roles`] poll system reads. Takes `Option<Res<AssetServer>>` so a
/// `MinimalPlugins` headless app with no [`AssetServer`] no-ops rather than panicking
/// (`bevy-traps.md` #1); under `DefaultPlugins` (the app + the `AssetServer` harness) the
/// load fires for real and the resolve runs.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_tile_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<TileRoles>>(TILE_ROLES_RON_PATH);
    commands.insert_resource(TileRolesHandle::new(handle));
}

/// `Update` (gated until [`TileRoles`] is resolved): resolve the loaded RON into the
/// presenter-owned [`TileRoles`] resource.
///
/// Once the [`RonAsset<TileRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<TileRoles>>` (a transient one-frame "loaded but not yet in the
/// collection" state simply leaves it un-inserted this pass — retried next frame), it
/// clones the deserialized [`TileRoles`] out and inserts it as the resident resource so
/// it is present before the first `BattleReady`. Run only while [`TileRolesHandle`]
/// exists AND [`TileRoles`] does NOT (the plugin's run-condition), so it inserts once.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert, [`Res<TileRolesHandle>`]
/// for the handle, [`Res<Assets<RonAsset<TileRoles>>>`] for the loaded asset.
pub fn resolve_tile_roles(
    mut commands: Commands,
    handle: Res<TileRolesHandle>,
    roles_assets: Res<Assets<RonAsset<TileRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until TileRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}

/// `Update` (unguarded; self-gates on its [`Option`] borrows): live-reload the resident
/// [`TileRoles`] resource when its `tile_roles.ron` is re-saved (GTW-375 C1).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`TileRoles`]`>>` — asset
/// events are MESSAGES in Bevy 0.19, so this is a `MessageReader`, not an `EventReader`
/// (`bevy-traps.md` #4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the in-flight
/// [`TileRolesHandle`]; events for any other handle are ignored. On a match it clones the
/// refreshed [`TileRoles`] out of the `Assets` collection and overwrites the resident
/// resource through [`ResMut`] — which MARKS it changed, so [`draw_static_battlefield`]'s
/// `roles.is_changed()` trigger re-renders the terrain tiles with the new indices the very
/// next frame, WITHOUT a rebuild. Mirrors `redrive_fx_tuning_on_asset_event`.
///
/// [`TileRoles`] is NOT `Copy` (it derives `Clone`), so the overwrite is `(**updated).clone()`.
///
/// Guarded so it never panics before the load chain has run (pre-resolve): it takes the
/// handle / the `Assets` collection / the [`TileRoles`] resource as [`Option`]al borrows,
/// draining the reader and returning early if any is missing (`bevy-traps.md` #1) so a
/// pre-resolve event does not linger and re-fire later.
///
/// [`draw_static_battlefield`]: crate::draw_static_battlefield
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional handle / `Assets` /
/// [`TileRoles`] borrows.
pub fn redrive_tile_roles_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<TileRoles>>>,
    handle: Option<Res<TileRolesHandle>>,
    roles_assets: Option<Res<Assets<RonAsset<TileRoles>>>>,
    roles: Option<ResMut<TileRoles>>,
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
    // TileRoles derives Clone (NOT Copy), so clone the refreshed table out of the asset.
    *roles = (**updated).clone();
    // GTW-374 Part C convention: log EVERY hot-reload path naming what reloaded, so a live
    // tile-role edit can be traced (mirrors the FX / combat / theme handlers).
    info!("tile hot-reload: re-resolved TileRoles from `sprites/tile_roles.spritedef.ron`");
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

    use super::{TileIndex, TileRoles, TileRolesHandle, redrive_tile_roles_on_asset_event};

    /// A scoped `tracing` layer that records each event's `message` field — the minimal
    /// capture needed to prove the `info!` hot-reload line fired. Mirrors the GTW-374
    /// combat redrive test's `CaptureLayer` (no shared util is reachable from this crate).
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

    /// A headless app with the real tile-role hot-reload wiring: `MinimalPlugins` +
    /// `AssetPlugin` (so `Assets<RonAsset<TileRoles>>` and the `AssetEvent` message buffer
    /// exist), the `TileRoles` RON loader registered, and the redrive system in `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset::<TileRoles>()
            .add_systems(Update, redrive_tile_roles_on_asset_event);
        app
    }

    /// A `TileRoles` whose `floor` index is `floor` and every other field is `7` — the
    /// distinct fixtures the re-resolve test contrasts.
    fn roles_with_floor(floor: usize) -> TileRoles {
        let other = TileIndex::new(7);
        TileRoles {
            floor:           TileIndex::new(floor),
            floor_alt_panel: other,
            wall:            other,
            wall_ew:         other,
            cover:           other,
            slab:            other,
            rubble:          other,
            slab_destroyed:  other,
            door:            other,
            stair_up:        other,
            stair_down:      other,
            ladder:          other,
        }
    }

    /// Add a `RonAsset<TileRoles>` to the collection and return its handle.
    fn add_asset(app: &mut App, roles: TileRoles) -> Handle<RonAsset<TileRoles>> {
        app.world_mut()
            .resource_mut::<Assets<RonAsset<TileRoles>>>()
            .add(RonAsset::new(roles))
    }

    /// Overwrite the in-memory payload of an already-added tile-role asset (the hot edit
    /// the file-watcher would make on a `tile_roles.ron` save).
    fn hot_edit(app: &mut App, handle: &Handle<RonAsset<TileRoles>>, roles: TileRoles) {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<TileRoles>>>();
        if let Some(mut asset) = assets.get_mut(handle) {
            **asset = roles;
        }
    }

    /// Inject an `AssetEvent::Modified` for the given handle id (standing in for the
    /// file-watcher's reload signal).
    fn inject_modified(app: &mut App, handle: &Handle<RonAsset<TileRoles>>) {
        app.world_mut()
            .write_message(AssetEvent::Modified { id: handle.id() });
    }

    /// C1/C8: on a matching `Modified` for the active tile-role handle, the redrive system
    /// re-resolves the resident `TileRoles` resource to the UPDATED in-memory payload — the
    /// live tile hot-reload. Driven through the real registered system via `app.update()`.
    ///
    /// Pin-discriminating: dropping the re-resolve leaves `TileRoles` on the OLD `floor`
    /// index; a wrong id filter would re-resolve on any handle's event.
    #[test]
    fn modified_event_reresolves_tile_roles() {
        let mut app = app();

        let baseline = roles_with_floor(6);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(TileRolesHandle::new(handle.clone()));

        // First update: no event, the resource is untouched.
        app.update();

        // Hot-edit the asset to a DISTINCT table, then fire a Modified.
        let edited = roles_with_floor(144);
        assert_ne!(edited, baseline, "precondition: the edit must differ");
        hot_edit(&mut app, &handle, edited.clone());
        inject_modified(&mut app, &handle);
        app.update();

        assert_eq!(
            app.world().get_resource::<TileRoles>(),
            Some(&edited),
            "a Modified for the active tile-role handle must re-resolve TileRoles to the new values",
        );
    }

    /// C8: a `Modified` for a DIFFERENT asset id leaves `TileRoles` untouched — the filter
    /// is on the ACTIVE handle id only.
    ///
    /// Pin-discriminating: dropping the id filter re-resolves on any tile-role asset's event.
    #[test]
    fn modified_event_for_other_id_does_not_reresolve() {
        let mut app = app();

        let baseline = roles_with_floor(6);
        let active = add_asset(&mut app, baseline.clone());
        // A second, unrelated tile-role asset whose value differs from the active one.
        let other = add_asset(&mut app, roles_with_floor(99));
        app.world_mut().insert_resource(baseline.clone());
        app.world_mut()
            .insert_resource(TileRolesHandle::new(active));

        app.update();
        // Edit the OTHER asset, fire Modified for it only.
        hot_edit(&mut app, &other, roles_with_floor(33));
        inject_modified(&mut app, &other);
        app.update();

        assert_eq!(
            app.world().get_resource::<TileRoles>(),
            Some(&baseline),
            "a Modified for a non-active tile-role id must NOT re-resolve TileRoles",
        );
    }

    /// C5/C8: the hot-reload `info!` line FIRES on the real re-resolve path, naming what
    /// reloaded. The redrive system is run via `run_system_once` on the calling thread
    /// (inside the scoped `tracing` subscriber) so the capture — which is thread-local —
    /// sees the emission; the schedule executor may run systems on a worker thread, which
    /// the capture would miss (GTW-374 thread-local capture lesson).
    ///
    /// Pin-discriminating: removing the `info!` from the redrive leaves the capture empty
    /// and this assert fails.
    #[test]
    fn hot_reload_logs_an_info_line() {
        let mut app = app();
        let baseline = roles_with_floor(6);
        let handle = add_asset(&mut app, baseline.clone());
        app.world_mut().insert_resource(baseline);
        app.world_mut()
            .insert_resource(TileRolesHandle::new(handle.clone()));
        // Stage the hot edit + the Modified message, then run the redrive synchronously
        // inside the capture scope.
        hot_edit(&mut app, &handle, roles_with_floor(144));
        inject_modified(&mut app, &handle);

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_tile_roles_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("tile hot-reload") && line.contains("TileRoles")),
            "the tile-role hot-reload must emit an info! line naming what reloaded; \
             captured: {captured:?}",
        );
    }
}
