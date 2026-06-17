//! The static-battlefield draw + cover-swap reaction, with the sim-grid bundle and the
//! presenter-owned sim-fact → tile-role mapping.

use bevy::{camera::visibility::RenderLayers, ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    BattleReady, Cell, CellLevel, CoverDestroyed, CoverLedger, GRID_HEIGHT, GRID_WIDTH,
    OccupancyGrid, SlabState, SurfaceGrid, TerrainKind,
};

use super::{
    active_level::ActiveLevel,
    roles::{TileIndex, TileRoles},
};
use crate::{CELL_PX, SheetRole, TopDownAtlases, cell_to_world};

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
pub(super) fn i32_extent(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
