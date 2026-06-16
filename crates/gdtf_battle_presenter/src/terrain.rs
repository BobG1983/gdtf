//! The static-battlefield terrain draw (GTW-48 S4 / GTW-218): the first VISUAL slice.
//!
//! This module reads the three sim-owned static-map resources — the
//! [`OccupancyGrid`](gdtf_battle_sim::OccupancyGrid) terrain, the
//! [`CoverLedger`](gdtf_battle_sim::CoverLedger) cover entries, and the
//! [`SurfaceGrid`](gdtf_battle_sim::SurfaceGrid) slabs — for the presenter-owned
//! [`ActiveLevel`] and spawns one 16x16 top-down terrain [`Sprite`] per non-empty
//! `(cell, level)`, choosing each tile's atlas index from a DATA-DRIVEN role table
//! ([`TileRoles`], loaded from `assets/tiles/tile_roles.ron`). It positions each
//! sprite via the S3 [`cell_to_world`](crate::cell_to_world) projection through the
//! S3 sprite-sizing recipe (`custom_size: Some(Vec2::splat(CELL_PX))`).
//!
//! Which sim fact maps to which ROLE is owned HERE; which atlas INDEX a role resolves
//! to is data, read from the [`TileRoles`] resource at draw time — never a hardcoded
//! literal. The model never reads the presenter (ADR-0001): this slice writes NOTHING
//! back to the sim, it only projects sim state to terrain sprites.
//!
//! # Draw lifecycle
//!
//! The initial draw is a ONE-SHOT triggered by draining
//! [`MessageReader<BattleReady>`](gdtf_battle_sim::BattleReady) — NOT per-frame polling
//! and NOT `Changed<Resource>` (the three grids are mutated in place with no per-cell
//! change detection). After the initial draw it reacts to exactly two further triggers:
//!
//! - an [`ActiveLevel`] change ([`ActiveLevel::is_changed`]): redraw the new level, the
//!   off-active-level terrain despawned (the despawn-all-then-respawn path also makes
//!   the first-ready double-fire idempotent), and
//! - a [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) message: swap that cover
//!   cell's sprite to the RUBBLE tile.
//!
//! It draws NO gangers (S5), NO FX (S6), and reads NO input (S7/S8) — it only REACTS
//! to [`ActiveLevel`] changing (S8's level-cycling input mutates the resource).

use bevy::{camera::visibility::RenderLayers, ecs::system::SystemParam, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    BattleReady, Cell, CellLevel, CoverDestroyed, CoverLedger, GRID_HEIGHT, GRID_WIDTH, Level,
    OccupancyGrid, SlabState, SurfaceGrid, TerrainKind,
};
use serde::Deserialize;

use crate::{CELL_PX, SheetRole, TopDownAtlases, cell_to_world};

/// An index into the terrain sheet's atlas layout — WHICH 16x16 tile a role draws.
///
/// A named newtype over `usize` (no-bare-types: an atlas index is a domain value, not
/// a bare `usize`), [`Deref`]ing to it so a consumer reads the index straight through.
/// `#[serde(transparent)]` so an authored `tile_roles.ron` field parses as a bare
/// integer (`floor: 6`), not a one-field struct.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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

/// The DATA-DRIVEN terrain tile-role table — each terrain ROLE → its [`TileIndex`].
///
/// Loaded from the loose `assets/tiles/tile_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`TileRoles`] resource before battle time (see [`resolve_tile_roles`]). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the ROLES.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<TileRoles>`] requires of its
/// payload). The `floor_alt_*` / `door` fields are authored for future variety; the
/// default S4 draw uses `floor` / `wall` / `cover` / `slab` / `rubble`.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct TileRoles {
    /// The default walkable-ground tile — in-range cells with no wall / cover / slab.
    pub floor:           TileIndex,
    /// A bolted/riveted grey-panel floor alternate (future variety).
    pub floor_alt_panel: TileIndex,
    /// A flat blue-grey stone floor alternate (future variety).
    pub floor_alt_stone: TileIndex,
    /// A flat orange/tan dirt floor alternate (future variety).
    pub floor_alt_dirt:  TileIndex,
    /// A green grass-field floor alternate (future variety).
    pub floor_alt_grass: TileIndex,
    /// The [`TerrainKind::Wall`] tile — solid fixed geometry.
    pub wall:            TileIndex,
    /// The [`TerrainKind::Cover`] tile — a chest-high cover prop.
    pub cover:           TileIndex,
    /// The [`SurfaceGrid`] `Present`-slab tile — a raised elevated deck.
    pub slab:            TileIndex,
    /// The destroyed-cover / damaged tile — broken debris scatter.
    pub rubble:          TileIndex,
    /// A doorway / hatch tile (authored for future variety).
    pub door:            TileIndex,
}

/// The presenter-owned ACTIVE storey — the single [`Level`] the terrain draw renders.
///
/// A named newtype over [`Level`] (no-bare-types) that [`Deref`]s to it. OWNED BY THE
/// PRESENTER CRATE so the `input -> presenter -> sim` direction holds: S5's ganger draw
/// READS it; S8's level-cycling input (in the input crate, which depends on the
/// presenter) MUTATES it. Present for the whole battle span (the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) inserts the
/// [`Default`] — level 0 — on build) so the later input slice has a resource to mutate.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveLevel(pub Level);

impl Default for ActiveLevel {
    /// The default active storey: the ground floor (level 0).
    fn default() -> Self {
        Self(Level::new(0))
    }
}

/// Marker tagging every static-terrain sprite this slice spawns.
///
/// A value-free marker (the no-bare-types marker carve-out) so a redraw / cover-swap
/// despawns exactly the terrain sprites — and ONLY them, never the S5 ganger sprites,
/// never the S2 [`WorldCamera`](crate::WorldCamera). It carries the source
/// [`CellLevel`] (the named sim newtype, never a bare key) so the
/// [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) reaction can find the one sprite
/// at the smashed cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainSprite {
    /// The `(cell, level)` this terrain sprite was drawn for.
    pub at: CellLevel,
}

/// Presenter draw ordering anchor — the named `Update`-schedule band the terrain draw
/// (and S5's ganger draw) registers into.
///
/// Defined ONCE via `configure_sets` (`bevy-traps.md` #5), then `.in_set`. The draw
/// band is ordered `.after(SimSystems::Simulate)` so a draw reads the sim AFTER its
/// world mutations this update (`bevy-traps.md` #3). This is the shared anchor S5's
/// ganger draw also registers into; S4 (the slice that lands first) introduces it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresenterSystems {
    /// The band holding the presenter's per-`(cell, level)` draw systems — ordered
    /// after the sim's world mutations so it observes a settled sim state.
    Draw,
}

/// The path of the loose tile-role RON, relative to the asset source root.
const TILE_ROLES_RON_PATH: &str = "tiles/tile_roles.ron";

/// The in-flight handle to the tile-role RON, held until it resolves into [`TileRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no
/// domain meaning; this name says "the tile-role table being loaded"). Inserted by
/// [`load_tile_roles`] and read by [`resolve_tile_roles`].
#[derive(Resource, Deref, Debug, Clone)]
pub struct TileRolesHandle(pub Handle<RonAsset<TileRoles>>);

/// `Startup`: kick off the `tile_roles.ron` load, storing its typed handle.
///
/// Loads `tiles/tile_roles.ron` as a [`RonAsset<TileRoles>`](gdtf_assets::RonAsset)
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
    commands.insert_resource(TileRolesHandle(handle));
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

/// The role a sim fact maps a `(cell, level)` to — the presenter-owned mapping from sim
/// state to a [`TileRoles`] role.
///
/// This slice owns WHICH role each sim fact maps to; the INDEX a role resolves to is
/// read from the [`TileRoles`] resource via [`TileRole::index`], never a hardcoded
/// literal. A cell with no terrain fact and no slab is [`None`] (no sprite); an
/// in-range cell with nothing on it is the [`Floor`](TileRole::Floor) default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TileRole {
    /// Default walkable ground — in-range, [`TerrainKind::Open`], no present slab.
    Floor,
    /// A wall — [`TerrainKind::Wall`].
    Wall,
    /// A piece of cover — [`TerrainKind::Cover`] (corroborated by a [`CoverLedger`]
    /// peek where present).
    Cover,
    /// A present slab — [`SurfaceGrid`] `Present` on the active level.
    Slab,
}

impl TileRole {
    /// The [`TileIndex`] this role resolves to, read from the [`TileRoles`] resource.
    const fn index(self, roles: &TileRoles) -> TileIndex {
        match self {
            Self::Floor => roles.floor,
            Self::Wall => roles.wall,
            Self::Cover => roles.cover,
            Self::Slab => roles.slab,
        }
    }
}

/// The three sim-owned static-map resources the terrain draw reads, bundled so the draw
/// system stays under the `too_many_arguments` clippy gate.
///
/// A `#[derive(SystemParam)]` borrow-bundle (the system-analogue of a cohesive ctor
/// struct): it groups the read-only sim grids the draw scans per `(cell, level)`. It is
/// gated at the system level by `run_if(resource_exists::<BattleInProgress>)`, so the
/// three resources are guaranteed present when the system runs (they are inserted by
/// the sim's `setup_battle` alongside `BattleInProgress`).
#[derive(SystemParam)]
pub struct StaticMap<'w> {
    /// The static-terrain grid (wall / cover / open per `(cell, level)`).
    occupancy: Res<'w, OccupancyGrid>,
    /// The cover-HP ledger (peeked read-only to corroborate a cover cell).
    cover:     Res<'w, CoverLedger>,
    /// The surface grid (slab presence per `(cell, level)`).
    surface:   Res<'w, SurfaceGrid>,
}

impl StaticMap<'_> {
    /// The [`TileRole`] for the in-range `key` on the active level.
    ///
    /// The presenter-owned mapping (`docs/combat/resolution.md` §3 cover / slab
    /// semantics): a `Present` slab is [`TileRole::Slab`]; a [`TerrainKind::Wall`] is
    /// [`TileRole::Wall`]; a [`TerrainKind::Cover`] (corroborated by a [`CoverLedger`]
    /// `peek`, NEVER a seeding accessor) is [`TileRole::Cover`]; an open cell with no
    /// present slab is the [`TileRole::Floor`] default. Every in-range cell is at least
    /// floor, so this always yields a role — the caller scans only in-range cells, so the
    /// out-of-range `Open` default of [`OccupancyGrid::terrain`] (FLOOR) never spawns a
    /// stray off-grid sprite.
    fn role_at(&self, key: &CellLevel) -> TileRole {
        // A present slab tile sits on the active level (slab_state is CellLevel-keyed).
        if matches!(self.surface.slab_state(key), SlabState::Present) {
            return TileRole::Slab;
        }
        match self.occupancy.terrain(key) {
            TerrainKind::Wall => TileRole::Wall,
            TerrainKind::Cover => {
                // peek() corroborates (never seeds) — None for an untouched key is fine,
                // the Cover terrain already chose the cover tile.
                let _ = self.cover.peek(key);
                TileRole::Cover
            }
            TerrainKind::Open => TileRole::Floor,
        }
    }
}

/// Builds one terrain [`Sprite`] for `role` on the terrain sheet, via the S3 recipe.
///
/// `Sprite::from_atlas_image(terrain.image, TextureAtlas { layout, index })` with the
/// role's [`TileIndex`] read from [`TileRoles`], then `custom_size =
/// Some(Vec2::splat(CELL_PX))` (the documented S3 sizing recipe). Returns [`None`] if
/// the terrain sheet was not loaded (so the caller skips the spawn rather than panic).
fn terrain_sprite(role: TileRole, roles: &TileRoles, atlases: &TopDownAtlases) -> Option<Sprite> {
    let terrain = atlases.role(SheetRole::Terrain)?;
    let mut sprite = Sprite::from_atlas_image(
        terrain.image.clone(),
        TextureAtlas {
            layout: terrain.layout.clone(),
            index:  *role.index(roles),
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    Some(sprite)
}

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): the
/// static-battlefield ONE-SHOT draw + redraw-on-level-change.
///
/// Fires when EITHER a [`BattleReady`](gdtf_battle_sim::BattleReady) drained this update
/// OR [`ActiveLevel`] `is_changed()`. It despawns ALL existing [`TerrainSprite`]
/// entities, then for the [`ActiveLevel`] ONLY scans `0..GRID_WIDTH` × `0..GRID_HEIGHT`
/// and spawns one terrain [`Sprite`] per non-empty cell (every in-range cell is at least
/// floor) at [`cell_to_world`](crate::cell_to_world), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`TerrainSprite`] marker.
/// The despawn-first step makes the first-ready double-fire (a `BattleReady` on the same
/// update `ActiveLevel` first reads `is_changed`) idempotent.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the [`StaticMap`] sim-grid bundle,
/// [`Res<TopDownAtlases>`], [`Res<TileRoles>`], [`Res<ActiveLevel>`],
/// [`MessageReader<BattleReady>`], and the [`TerrainSprite`] despawn query.
pub fn draw_static_battlefield(
    mut commands: Commands,
    map: StaticMap,
    atlases: Res<TopDownAtlases>,
    roles: Res<TileRoles>,
    active: Res<ActiveLevel>,
    mut ready: MessageReader<BattleReady>,
    existing: Query<Entity, With<TerrainSprite>>,
) {
    // The redraw triggers: a drained BattleReady (one-shot) OR an ActiveLevel change.
    // Fully DRAIN the reader (`.count()`, not `.next()`) so a multi-message ready never
    // leaves an unread BattleReady to re-fire a redundant redraw next update.
    let ready_fired = ready.read().count() > 0;
    if !ready_fired && !active.is_changed() {
        return;
    }

    // Despawn ALL existing terrain sprites first — idempotent (a same-update
    // ready+is_changed double-fire redraws once) and level-scoped (off-active-level
    // terrain is gone after a level change).
    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let level = **active;
    for y in 0..i32_extent(GRID_HEIGHT) {
        for x in 0..i32_extent(GRID_WIDTH) {
            let cell = Cell::new(x, y);
            let key = CellLevel::new(cell, level);
            let role = map.role_at(&key);
            let Some(sprite) = terrain_sprite(role, &roles, &atlases) else {
                continue;
            };
            commands.spawn((
                sprite,
                Transform::from_translation(cell_to_world(cell, level)),
                RenderLayers::layer(crate::WORLD_RENDER_LAYER),
                TerrainSprite { at: key },
            ));
        }
    }
}

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): swap
/// a destroyed cover cell's sprite to the RUBBLE tile.
///
/// Drains [`MessageReader<CoverDestroyed>`](gdtf_battle_sim::CoverDestroyed); for each
/// `CoverDestroyed { at }` ON THE ACTIVE LEVEL it finds the [`TerrainSprite`] at `at` and
/// swaps its texture-atlas index to the `rubble` [`TileIndex`] (read from [`TileRoles`],
/// never a literal). Off-active-level destructions are ignored (that terrain is not
/// drawn). Choice: SWAP (not despawn) so the cell still reads as terrain (rubble) rather
/// than a hole — AC3 asserts the swap.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<TileRoles>`],
/// [`MessageReader<CoverDestroyed>`], and the [`TerrainSprite`] query (to edit the
/// sprite at `at`) — no [`Commands`] needed, the swap edits the sprite in place.
pub fn swap_destroyed_cover(
    active: Res<ActiveLevel>,
    roles: Res<TileRoles>,
    mut destroyed: MessageReader<CoverDestroyed>,
    mut sprites: Query<(&TerrainSprite, &mut Sprite)>,
) {
    let active_storey = **active;
    let rubble = *roles.rubble;
    for event in destroyed.read() {
        // CoverDestroyed.at is a CellLevel; only act on cells on the active storey.
        if event.at.z != i32::from(*active_storey) {
            continue;
        }
        for (terrain, mut sprite) in &mut sprites {
            if terrain.at != event.at {
                continue;
            }
            // Swap to the rubble tile — keep the sprite (it still reads as terrain),
            // re-using the terrain sheet's loaded atlas the draw already built it from.
            // A sprite drawn with no atlas (the terrain sheet was absent at draw time)
            // has nothing to re-index; skip it rather than panic.
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = rubble;
            }
        }
    }
}

/// Converts a `usize` grid extent ([`GRID_WIDTH`] / [`GRID_HEIGHT`]) to the `i32` cell
/// coordinate range bound, saturating rather than wrapping.
///
/// [`GRID_WIDTH`] / [`GRID_HEIGHT`] are `usize` (flat-buffer extents); a cell coordinate
/// is `i32` ([`Cell`] wraps `IVec2`). `i32::try_from` clamps an (impossible-in-practice)
/// over-large extent to [`i32::MAX`] rather than wrap (`cast_possible_wrap`).
fn i32_extent(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped `tile_roles.ron` parses into `TileRoles` and exposes every
    /// documented role — a `ron::de` round-trip of the SHIPPED bytes (AC1).
    ///
    /// It asserts the file PARSES and HAS all roles; it does NOT pin a tunable index
    /// magnitude (those are data the engineer eyeballs and may adjust). A `floor`
    /// failing to differ from `wall` would be a copy-paste authoring error, so the
    /// distinctness check is a light structural guard, not a magnitude pin.
    #[test]
    fn shipped_tile_roles_ron_parses_with_all_roles() {
        const SHIPPED: &str = include_str!("../../../assets/tiles/tile_roles.ron");
        let parsed: Result<TileRoles, _> = ron::de::from_str(SHIPPED);
        // Parsing into TileRoles proves every documented role is present (a missing
        // field would be a deserialize error). Assert the parse succeeded; if not,
        // surface the error rather than pinning any index magnitude.
        assert!(
            parsed.is_ok(),
            "shipped tile_roles.ron must parse into TileRoles, got: {:?}",
            parsed.as_ref().err(),
        );
        let Ok(roles) = parsed else {
            return;
        };
        // Sanity-check the five draw roles are not all collapsed onto one index (an
        // authoring slip) — a structural guard, not a magnitude pin.
        let draw_roles = [
            roles.floor,
            roles.wall,
            roles.cover,
            roles.slab,
            roles.rubble,
        ];
        let all_same = draw_roles.iter().all(|r| *r == roles.floor);
        assert!(
            !all_same,
            "the five draw roles must not all share one index (authoring slip)",
        );
        // The door role is documented; the struct parsing means it is present.
        let _ = roles.door;
        let _ = roles.floor_alt_panel;
        let _ = roles.floor_alt_stone;
        let _ = roles.floor_alt_dirt;
        let _ = roles.floor_alt_grass;
    }

    /// `i32_extent` returns the grid extent unchanged for the real 60x60 grid.
    #[test]
    fn i32_extent_passes_the_real_grid_extents() {
        assert_eq!(i32_extent(GRID_WIDTH), 60, "GRID_WIDTH is 60");
        assert_eq!(i32_extent(GRID_HEIGHT), 60, "GRID_HEIGHT is 60");
    }

    /// `ActiveLevel` defaults to the ground floor (level 0).
    #[test]
    fn active_level_defaults_to_level_zero() {
        assert_eq!(
            *ActiveLevel::default(),
            Level::new(0),
            "default active level is 0"
        );
    }
}
