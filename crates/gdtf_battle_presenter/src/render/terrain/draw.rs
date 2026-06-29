//! The static-battlefield draw + cover-swap reaction, with the sim-grid bundle and the
//! presenter-owned sim-fact → tile-role mapping.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{system::SystemParam, template::template},
    image::TextureAtlasLayout,
    math::primitives::Rectangle,
    platform::collections::HashMap,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    BattleReady, Cell, CellLevel, CoverDestroyed, CoverLedger, FootfallSound, GRID_HEIGHT,
    GRID_WIDTH, OccupancyGrid, SlabDestroyed, SlabState, SurfaceGrid, TerrainCell,
    TerrainGraphicKey, TerrainKind,
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

/// The sim-owned static-map state the terrain draw reads, bundled so the draw system
/// stays under the `too_many_arguments` clippy gate.
///
/// A `#[derive(SystemParam)]` borrow-bundle (the system-analogue of a cohesive ctor
/// struct): it groups the read-only sim grids the draw scans per `(cell, level)` PLUS the
/// per-terrain-entity presentation-fact query (GTW-493). The three GRIDS are gated at the
/// system level by `run_if(resource_exists::<BattleInProgress>)`, so they are guaranteed
/// present when the system runs (they are inserted by the sim's `setup_battle` alongside
/// `BattleInProgress`). The terrain-entity query (GTW-493) reads the
/// [`TerrainGraphicKey`] the sim spawns on every terrain entity (ALL kinds incl. `Wall`)
/// plus the OPTIONAL slab-only [`FootfallSound`] — the per-def presentation facts.
#[derive(SystemParam)]
pub struct StaticMap<'w, 's> {
    /// The static-terrain grid (wall / cover / open per `(cell, level)`).
    occupancy: Res<'w, OccupancyGrid>,
    /// The cover-HP ledger (peeked read-only to corroborate a cover cell).
    cover:     Res<'w, CoverLedger>,
    /// The surface grid (slab presence per `(cell, level)`).
    surface:   Res<'w, SurfaceGrid>,
    /// The per-terrain-entity presentation facts the sim spawns (GTW-493): every terrain
    /// entity's `(cell, level)` ([`TerrainCell`]), its per-def graphic key
    /// ([`TerrainGraphicKey`], ALL kinds incl. `Wall`), and the OPTIONAL slab-only
    /// footfall ([`FootfallSound`]). The presenter reads these facts ONLY — never the
    /// sim-owned `TerrainTag` (the one-way sim→presenter dependency forbids it).
    terrain: Query<
        'w,
        's,
        (
            &'static TerrainCell,
            &'static TerrainGraphicKey,
            Option<&'static FootfallSound>,
        ),
    >,
}

impl StaticMap<'_, '_> {
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
    ///
    /// This is the FALLBACK keyed only on [`TerrainKind`] — the GTW-493 per-def
    /// graphic-key resolution (the private `resolve_index` helper →
    /// [`TileRoles::index_for_key`]) is tried FIRST; this role default applies only to a
    /// cell with no spawned terrain entity (the floor field) or an out-of-vocabulary
    /// graphic key.
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

    /// Build the per-cell terrain presentation-fact map (GTW-493) from the spawned
    /// terrain entities: `(cell, level)` → (per-def [`TerrainGraphicKey`], OPTIONAL slab
    /// [`FootfallSound`]).
    ///
    /// One pass over the terrain-entity query, built ONCE per draw and consulted per
    /// cell. The sim spawns at most one terrain entity per `(cell, level)`
    /// (`setup_battle`'s walls/scatter/slab loops), so a later insert merely overwrites —
    /// no terrain stacks. A cell with no spawned entity (the floor field) is absent from
    /// the map and falls back to the [`role_at`](Self::role_at) `TerrainKind` default.
    fn graphic_facts(&self) -> HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)> {
        self.terrain
            .iter()
            .map(|(cell, graphic, footfall)| (**cell, (graphic, footfall)))
            .collect()
    }
}

/// Builds one terrain [`TerrainFogMaterial`] for the resolved `index` on the terrain
/// sheet (GTW-348).
///
/// Takes the already-resolved [`TileIndex`] (GTW-493 — the caller resolves it from the
/// cell's per-def [`TerrainGraphicKey`] first, falling back to the
/// [`TileRole`]-table default keyed on [`TerrainKind`]) and the terrain sheet's RESOLVED
/// [`TextureAtlasLayout`] (looked up from the handle via
/// [`Assets<TextureAtlasLayout>::get`](bevy::asset::Assets::get) — the material holds the
/// layout STRUCT, not the handle, so its [`AsBindGroupShaderType`](bevy::render::render_resource::AsBindGroupShaderType)
/// can bake the atlas UV). `custom_size = Some(Vec2::splat(CELL_PX))` (the S3 sizing
/// recipe) and `saturation = 1.0` (seeded VISIBLE — the fog writer drives it per cell).
/// Returns [`None`] if the terrain sheet OR its layout was not loaded (so the caller skips
/// the spawn rather than panic).
fn terrain_material(
    index: TileIndex,
    atlases: &TopDownAtlases,
    layouts: &Assets<TextureAtlasLayout>,
) -> Option<TerrainFogMaterial> {
    let terrain = atlases.role(SheetRole::Terrain)?;
    let atlas_layout = layouts.get(&terrain.layout)?.clone();
    Some(TerrainFogMaterial {
        image:        terrain.image.clone(),
        atlas_layout: Some(atlas_layout),
        atlas_index:  *index,
        custom_size:  Some(Vec2::splat(CELL_PX)),
        // Seed VISIBLE (full colour); present_fog drives it to 0.0 on EXPLORED cells.
        saturation:   1.0,
    })
}

/// Resolve the atlas [`TileIndex`] for an in-range cell (GTW-493) — the per-def
/// [`TerrainGraphicKey`] FIRST, the [`TileRole`]-table default LAST.
///
/// The GTW-493 seam: the sim spawns a [`TerrainGraphicKey`] on every terrain entity (ALL
/// kinds incl. `Wall`), keyed in the [`TileRoles`] vocabulary. If a terrain entity sits at
/// `key`, its graphic key resolves through [`TileRoles::index_for_key`] — so two `Cover`
/// defs whose `graphic_name`s differ (e.g. `"cover"` vs `"rubble"`) draw DISTINCT sprites
/// (the per-def graphic the ticket requires; the role-table default, keyed only on the
/// shared [`TerrainKind::Cover`], could not). A cell with no spawned terrain entity (the
/// floor field) or an out-of-vocabulary graphic key falls back to the
/// [`StaticMap::role_at`] default keyed on [`TerrainKind`] / slab presence — so an
/// unrecognized key still draws (no panic) rather than vanishing.
fn resolve_index(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)>,
    map: &StaticMap,
    roles: &TileRoles,
) -> TileIndex {
    // GTW-493: per-def graphic FIRST. A spawned terrain entity carries the def's
    // graphic_name; resolve it against the TileRoles vocabulary. Falls through to the
    // TerrainKind-keyed role default for the floor field (no entity) or an
    // out-of-vocabulary key (index_for_key -> None).
    if let Some((graphic, _footfall)) = facts.get(key)
        && let Some(index) = roles.index_for_key(graphic)
    {
        return index;
    }
    map.role_at(key).index(roles)
}

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): the
/// static-battlefield ONE-SHOT draw + redraw-on-level-change.
///
/// Fires when ANY of: a [`BattleReady`](gdtf_battle_sim::BattleReady) drained this update,
/// [`ActiveLevel`] `is_changed()`, OR [`TileRoles`] `is_changed()` — the GTW-375 third
/// trigger that re-renders the terrain on a tile hot-reload. A `tile_roles.ron` re-save
/// MUTATES [`TileRoles`] (the indices swap) and an `alt_tileset_terrain.png` re-save
/// `set_changed()`s it (same indices, fresh GPU texture); either way the rendered tiles
/// SWAP, live, with no restart. It despawns ALL existing [`TerrainSprite`]
/// entities, then for the [`ActiveLevel`] ONLY scans `0..GRID_WIDTH` × `0..GRID_HEIGHT`
/// and spawns one terrain tile per non-empty cell (every in-range cell is at least floor)
/// as a shared unit-rect [`Mesh2d`] + [`MeshMaterial2d<TerrainFogMaterial>`] (GTW-348 — the
/// material path so EXPLORED can render greyscale; the `Sprite` pipeline cannot desaturate)
/// at [`cell_to_world`](crate::cell_to_world), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`TerrainSprite`] marker.
/// The despawn-first step makes the first-ready double-fire (a `BattleReady` on the same
/// update `ActiveLevel` first reads `is_changed`) idempotent.
///
/// GTW-493 (T07c — the per-def presenter seam): each cell's atlas index is resolved from
/// the SIM-SPAWNED terrain entity's per-def [`TerrainGraphicKey`] FIRST (via the private
/// `resolve_index` helper → [`TileRoles::index_for_key`]), falling back to the
/// `TileRole`-table default keyed on [`TerrainKind`] only for the floor field (no spawned
/// entity) or an out-of-vocabulary key. So two `Cover` defs whose `graphic_name`s differ
/// (e.g. `"cover"` vs `"rubble"`) draw DISTINCT sprites — the per-def graphic the role
/// table (keyed only on the shared [`TerrainKind`]) cannot express. The slab-only OPTIONAL
/// [`FootfallSound`] is also read here from the def's presenter facts; an absent footfall is
/// handled (no panic) with a documented silent default — there is no footfall-audio system
/// yet (guns-only).
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
    // The redraw triggers: a drained BattleReady (one-shot), an ActiveLevel change, OR a
    // changed TileRoles (GTW-375). The TileRoles trigger is THE single redraw signal for a
    // tile-appearance hot-reload — both reload paths converge on it: a `tile_roles.ron`
    // re-save MUTATES TileRoles (redrive_tile_roles_on_asset_event), and an
    // `alt_tileset_terrain.png` re-save `set_changed()`s it (redrive_terrain_sheet_on_asset_event)
    // so the tiles re-render against the freshly-reloaded GPU texture. present_fog runs
    // `.after(draw_static_battlefield)`, so the fog re-applies to the redrawn tiles.
    // Fully DRAIN the reader (`.count()`, not `.next()`) so a multi-message ready never
    // leaves an unread BattleReady to re-fire a redundant redraw next update.
    let ready_fired = ready.read().count() > 0;
    if !ready_fired && !active.is_changed() && !roles.is_changed() {
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

    // GTW-493: build the per-cell terrain presentation-fact map ONCE per draw — the
    // sim-spawned `(cell, level)` -> (per-def TerrainGraphicKey, optional slab FootfallSound).
    // Each cell then resolves its atlas index from its OWN def's graphic (resolve_index),
    // not solely from the TerrainKind-keyed TileRole default.
    let graphic_facts = map.graphic_facts();

    let level = **active;
    for y in 0..i32_extent(GRID_HEIGHT) {
        for x in 0..i32_extent(GRID_WIDTH) {
            let cell = Cell::new(x, y);
            let key = CellLevel::new(cell, level);
            // GTW-493: read the slab footfall from the def's presenter_kind for this cell.
            // It is Slab-ONLY and OPTIONAL: a Wall/Cover entity carries no FootfallSound, and
            // a Slab def may omit it. ABSENT footfall is handled here with NO panic and a
            // DOCUMENTED default — there is NO footfall-audio system yet (the sim stays
            // guns-only), so a present key is logged at `debug` (so a future footfall pass has
            // a wired seam to consume) and an absent one is the SILENT default (no clip).
            if let Some((_graphic, Some(footfall))) = graphic_facts.get(&key) {
                let footfall_key: &str = footfall;
                debug!(
                    "terrain footfall present at {key:?}: `{footfall_key}` (no footfall-audio \
                     system yet — default: silent)",
                );
            }
            // GTW-493: resolve the atlas index from this cell's per-def TerrainGraphicKey
            // FIRST (so two same-TerrainKind defs with distinct graphic_names draw distinct
            // sprites), falling back to the TerrainKind-keyed TileRole default for the floor
            // field / an out-of-vocabulary key.
            let index = resolve_index(&key, &graphic_facts, &map, &roles);
            let Some(material) = terrain_material(index, &atlases, &layouts) else {
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

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): swap a
/// destroyed-SLAB cell's sprite to the destroyed-slab tile.
///
/// The slab mirror of [`swap_destroyed_cover`] (GTW-367 C1/C3): it drains
/// [`MessageReader<SlabDestroyed>`](gdtf_battle_sim::SlabDestroyed); for each
/// `SlabDestroyed { at }` ON THE ACTIVE LEVEL it finds the [`TerrainSprite`] at `at` and
/// swaps its texture-atlas index to the `slab_destroyed` [`TileIndex`] (read from
/// [`TileRoles`], never a literal — the engineer's-choice destroyed-slab treatment, which
/// the [`TileRoles::slab_destroyed`] doc-comment describes and flags for art review).
/// Off-active-level destructions are ignored (that terrain is not drawn — the hard cut to
/// [`ActiveLevel`], C4). Choice: SWAP (not despawn) so the cell still reads as terrain
/// (rubble/debris) rather than a hole, and so the SAME sprite `Entity` persists across the
/// swap (the UI mutate-not-respawn rule, C7) — exactly the cover path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<TileRoles>`],
/// [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap re-indexes the tile's material
/// rather than its sprite), [`MessageReader<SlabDestroyed>`], and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] query (to find the material at `at`) — no [`Commands`] needed, the
/// swap edits the material in place (no despawn / respawn).
///
/// [`TileRoles::slab_destroyed`]: super::roles::TileRoles::slab_destroyed
pub fn swap_destroyed_slab(
    active: Res<ActiveLevel>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<SlabDestroyed>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let active_storey = **active;
    let slab_destroyed = *roles.slab_destroyed;
    for event in destroyed.read() {
        // SlabDestroyed.at is a CellLevel; only act on cells on the active storey (C4 hard
        // cut — an off-active-level slab is not drawn, so nothing to re-index there).
        if event.at.z != i32::from(*active_storey) {
            continue;
        }
        for (terrain, mat_handle) in &tiles {
            if terrain.at != event.at {
                continue;
            }
            // Re-index to the destroyed-slab tile — keep the entity (it still reads as
            // terrain). get_mut marks the material asset dirty so the UV-transform uniform
            // re-uploads next frame (mirroring swap_destroyed_cover's in-place edit). A tile
            // whose material was dropped (the terrain sheet was absent at draw time) is simply
            // not in the query; nothing to re-index.
            if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                material.atlas_index = slab_destroyed;
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
