//! The static-battlefield draw + cover-swap reaction, with the sim-grid bundle and the
//! presenter-owned sim-fact → tile-role mapping.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{system::SystemParam, template::template},
    image::TextureAtlasLayout,
    math::primitives::Rectangle,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    BattleReady, Cell, CellLevel, CoverDestroyed, CoverLedger, GRID_HEIGHT, GRID_WIDTH,
    OccupancyGrid, SlabState, SurfaceGrid, TerrainKind,
};

use super::{
    active_level::ActiveLevel,
    roles::{TileIndex, TileRoles},
};
use crate::{CELL_PX, SheetRole, TerrainFogMaterial, TopDownAtlases, cell_to_world};

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

/// Builds one terrain [`TerrainFogMaterial`] for `role` on the terrain sheet (GTW-348).
///
/// Resolves the role's [`TileIndex`] from [`TileRoles`] and the terrain sheet's RESOLVED
/// [`TextureAtlasLayout`] (looked up from the handle via
/// [`Assets<TextureAtlasLayout>::get`](bevy::asset::Assets::get) — the material holds the
/// layout STRUCT, not the handle, so its [`AsBindGroupShaderType`](bevy::render::render_resource::AsBindGroupShaderType)
/// can bake the atlas UV). `custom_size = Some(Vec2::splat(CELL_PX))` (the S3 sizing
/// recipe) and `saturation = 1.0` (seeded VISIBLE — the fog writer drives it per cell).
/// Returns [`None`] if the terrain sheet OR its layout was not loaded (so the caller skips
/// the spawn rather than panic).
fn terrain_material(
    role: TileRole,
    roles: &TileRoles,
    atlases: &TopDownAtlases,
    layouts: &Assets<TextureAtlasLayout>,
) -> Option<TerrainFogMaterial> {
    let terrain = atlases.role(SheetRole::Terrain)?;
    let atlas_layout = layouts.get(&terrain.layout)?.clone();
    Some(TerrainFogMaterial {
        image:        terrain.image.clone(),
        atlas_layout: Some(atlas_layout),
        atlas_index:  *role.index(roles),
        custom_size:  Some(Vec2::splat(CELL_PX)),
        // Seed VISIBLE (full colour); present_fog drives it to 0.0 on EXPLORED cells.
        saturation:   1.0,
    })
}

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): the
/// static-battlefield ONE-SHOT draw + redraw-on-level-change.
///
/// Fires when EITHER a [`BattleReady`](gdtf_battle_sim::BattleReady) drained this update
/// OR [`ActiveLevel`] `is_changed()`. It despawns ALL existing [`TerrainSprite`]
/// entities, then for the [`ActiveLevel`] ONLY scans `0..GRID_WIDTH` × `0..GRID_HEIGHT`
/// and spawns one terrain tile per non-empty cell (every in-range cell is at least floor)
/// as a shared unit-rect [`Mesh2d`] + [`MeshMaterial2d<TerrainFogMaterial>`] (GTW-348 — the
/// material path so EXPLORED can render greyscale; the `Sprite` pipeline cannot desaturate)
/// at [`cell_to_world`](crate::cell_to_world), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`TerrainSprite`] marker.
/// The despawn-first step makes the first-ready double-fire (a `BattleReady` on the same
/// update `ActiveLevel` first reads `is_changed`) idempotent.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the [`StaticMap`] sim-grid bundle,
/// [`Res<TopDownAtlases>`], [`Res<TileRoles>`], [`Res<ActiveLevel>`], the asset stores it
/// builds tiles from ([`ResMut<Assets<TerrainFogMaterial>>`] for the per-tile material,
/// [`ResMut<Assets<Mesh>>`] + a [`Local`] cache for the shared unit-rect quad,
/// [`Res<Assets<TextureAtlasLayout>>`] to resolve the atlas layout — GTW-348),
/// [`MessageReader<BattleReady>`], and the [`TerrainSprite`] despawn query.
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-348 material path adds three asset stores (TerrainFogMaterial, Mesh, \
              the atlas-layout resolve) to the existing draw params; grouping into a \
              SystemParam bundle would not reduce the count and would obscure the per-arg docs"
)]
pub fn draw_static_battlefield(
    mut commands: Commands,
    map: StaticMap,
    atlases: Res<TopDownAtlases>,
    roles: Res<TileRoles>,
    active: Res<ActiveLevel>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
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

    // The shared unit-rect quad (Rectangle::from_size(1×1), the same mesh Bevy's
    // SpriteMeshPlugin builds): added ONCE, then reused for every tile. The per-tile
    // TerrainFogMaterial's vertex_scale (= custom_size) sizes the quad in the shader.
    let mesh = quad
        .get_or_insert_with(|| meshes.add(Rectangle::from_size(Vec2::ONE)))
        .clone();

    let level = **active;
    for y in 0..i32_extent(GRID_HEIGHT) {
        for x in 0..i32_extent(GRID_WIDTH) {
            let cell = Cell::new(x, y);
            let key = CellLevel::new(cell, level);
            let role = map.role_at(&key);
            let Some(material) = terrain_material(role, &roles, &atlases, &layouts) else {
                continue;
            };
            let mesh2d = Mesh2d(mesh.clone());
            let material2d = MeshMaterial2d(materials.add(material));
            let transform = Transform::from_translation(cell_to_world(cell, level));
            let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
            // GTW-348 — terrain moved from the `Sprite` path to a `Mesh2d` +
            // `MeshMaterial2d<TerrainFogMaterial>` so EXPLORED cells can render GREYSCALE
            // (the sprite pipeline's per-channel multiply tint cannot desaturate). Authored
            // via `spawn_scene` (GTW-322): `Mesh2d` / `MeshMaterial2d` each wrap a `Handle<_>`,
            // which is NOT `Unpin` (so it has no `Template` impl) — exactly like the old atlas
            // `Sprite` — so each rides the `template(move |_| Ok(value.clone()))` closure escape
            // hatch (the `FnTemplate` output carries no `Unpin` bound), the same one the AREA-1
            // widget builders use. The runtime `Transform` / `RenderLayers` ARE
            // `Clone + Default + Unpin`, so each rides `template_value` (a value-overwrite). The
            // `TerrainSprite` marker carries the runtime source `CellLevel` (no `Default`), so it
            // is `.insert`ed after the scene. The tile is never read back by id (the redraw /
            // cover-swap / fog find it via the `TerrainSprite { at }` query, not a captured
            // handle), so the deferred materialization is inert — the same entity + components
            // result.
            commands
                .spawn_scene((
                    bsn! { template(move |_| Ok(mesh2d.clone())) },
                    bsn! { template(move |_| Ok(material2d.clone())) },
                    template_value(transform),
                    template_value(layers),
                ))
                .insert(TerrainSprite { at: key });
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
/// [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap re-indexes the tile's
/// material rather than its sprite), [`MessageReader<CoverDestroyed>`], and the
/// [`TerrainSprite`] / [`MeshMaterial2d`] query (to find the material at `at`) — no
/// [`Commands`] needed, the swap edits the material in place (no despawn / respawn).
pub fn swap_destroyed_cover(
    active: Res<ActiveLevel>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<CoverDestroyed>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let active_storey = **active;
    let rubble = *roles.rubble;
    for event in destroyed.read() {
        // CoverDestroyed.at is a CellLevel; only act on cells on the active storey.
        if event.at.z != i32::from(*active_storey) {
            continue;
        }
        for (terrain, mat_handle) in &tiles {
            if terrain.at != event.at {
                continue;
            }
            // Re-index to the rubble tile — keep the entity (it still reads as terrain).
            // get_mut marks the material asset dirty so the UV-transform uniform re-uploads
            // next frame (mirroring the fog writer's in-place saturation edit). A tile whose
            // material was dropped (the terrain sheet was absent at draw time, so none
            // spawned) is simply not in the query; nothing to re-index.
            if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                material.atlas_index = rubble;
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
