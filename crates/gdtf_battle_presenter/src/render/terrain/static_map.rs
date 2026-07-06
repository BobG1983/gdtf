//! The sim-fact read model: the [`StaticMap`] borrow-bundle, the role / index
//! resolution, the per-tile material build, and the grid-extent util.

use bevy::{
    ecs::system::SystemParam, image::TextureAtlasLayout, platform::collections::HashMap, prelude::*,
};
use gdtf_battle_sim::{
    cover::CoverLedger,
    entity::TerrainCell,
    occupancy::TerrainKind,
    piece::{FootfallSound, TerrainGraphicKey},
    prelude::{CellLevel, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
};

use super::roles::{TileIndex, TileRole, TileRoles};
use crate::{Brightness, CELL_PX, SheetRole, TerrainFogMaterial, TopDownAtlases};

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
    /// The presenter-owned sim-fact → role mapping (`docs/combat/resolution.md` §3
    /// cover / slab semantics) over the shared GTW-566 [`TileRole`] vocabulary (this
    /// mapping is the one place the fallback CHOICE lives; the INDEX a role resolves to
    /// is read from [`TileRoles`] via [`TileRole::index_in`], never a hardcoded
    /// literal): a `Present` slab is [`TileRole::Slab`]; a [`TerrainKind::Wall`] is
    /// [`TileRole::Wall`]; a [`TerrainKind::Cover`] (corroborated by a [`CoverLedger`]
    /// `peek`, NEVER a seeding accessor) is [`TileRole::Cover`]; an open cell with no
    /// present slab is the [`TileRole::Floor`] default. Every in-range cell is at least
    /// floor, so this always yields a role — the caller scans only in-range cells, so the
    /// out-of-range `Open` default of [`OccupancyGrid::terrain`] (FLOOR) never spawns a
    /// stray off-grid sprite.
    ///
    /// This is the FALLBACK keyed only on [`TerrainKind`] — the GTW-493 per-def
    /// graphic-key resolution (the [`resolve_index`] helper →
    /// [`TileRole::from_key`]) is tried FIRST; this role default applies only to a
    /// cell with no spawned terrain entity (the floor field) or an out-of-vocabulary
    /// graphic key.
    pub(super) fn role_at(&self, key: &CellLevel) -> TileRole {
        // A present slab tile sits on the active level (slab_state is CellLevel-keyed).
        if matches!(self.surface.slab_state(key), SlabState::Present) {
            return TileRole::Slab;
        }
        match self.occupancy.terrain(key) {
            TerrainKind::Wall => TileRole::Wall,
            // GTW-543: an emplacement is a cover-like smashable structure — it falls back to
            // the Cover role here (the per-def graphic-key resolution via GTW-493
            // `resolve_index` fires FIRST and picks the authored emplacement sprite by its
            // `graphic_name`, so this role default only applies to a cell with no spawned
            // emplacement entity).
            TerrainKind::Cover | TerrainKind::Emplacement => {
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
    pub(super) fn graphic_facts(
        &self,
    ) -> HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)> {
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
/// recipe), `saturation = 1.0` (seeded VISIBLE — the fog writer drives it per cell), and
/// `brightness = `[`Brightness::FULL`] (seeded full-bright — the fog writer dims it on a
/// lower drawn storey, GTW-519). Returns [`None`] if the terrain sheet OR its layout was not
/// loaded (so the caller skips the spawn rather than panic).
pub(super) fn terrain_material(
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
        // GTW-519: seed full brightness; present_fog dims a lower drawn storey per-tile.
        brightness:   Brightness::FULL,
    })
}

/// Resolve the atlas [`TileIndex`] for an in-range cell (GTW-493) — the per-def
/// [`TerrainGraphicKey`] FIRST, the [`TileRole`]-table default LAST.
///
/// The GTW-493 seam: the sim spawns a [`TerrainGraphicKey`] on every terrain entity (ALL
/// kinds incl. `Wall`), keyed in the [`TileRoles`] vocabulary. If a terrain entity sits at
/// `key`, its graphic key classifies through [`TileRole::from_key`] and resolves via
/// [`TileRole::index_in`] — so two `Cover` defs whose `graphic_name`s differ (e.g.
/// `"cover"` vs `"rubble"`) draw DISTINCT sprites (the per-def graphic the ticket
/// requires; the role-table default, keyed only on the shared [`TerrainKind::Cover`],
/// could not). A cell with no spawned terrain entity (the floor field) falls back
/// silently to the [`StaticMap::role_at`] default keyed on [`TerrainKind`] / slab
/// presence; an OUT-OF-VOCABULARY graphic key takes the same no-panic fallback but is
/// LOUD about it (GTW-566 C4) — a `warn!` names the unresolvable key and the cell, so an
/// authored typo surfaces in the log instead of silently drawing the role default.
pub(super) fn resolve_index(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)>,
    map: &StaticMap,
    roles: &TileRoles,
) -> TileIndex {
    // GTW-493: per-def graphic FIRST. A spawned terrain entity carries the def's
    // graphic_name; classify it against the TileRole vocabulary. Falls through to the
    // TerrainKind-keyed role default for the floor field (no entity) or an
    // out-of-vocabulary key (from_key -> None — warned, GTW-566 C4).
    if let Some((graphic, _footfall)) = facts.get(key) {
        if let Some(role) = TileRole::from_key(graphic) {
            return role.index_in(roles);
        }
        // GTW-566 C4: a spawned key that fails classification is an authored typo (or
        // a def authored against a newer vocabulary) — say so loudly, naming the key
        // and the cell, then keep the no-panic role-default fallback draw.
        let unresolvable: &str = graphic;
        warn!(
            "terrain draw: unresolvable graphic key `{unresolvable}` at {key:?} — not in \
             the TileRoles vocabulary; drawing the TerrainKind role-default tile instead",
        );
    }
    map.role_at(key).index_in(roles)
}

/// Whether `key`'s storey has REAL terrain the multi-level draw emits a sprite for (GTW-519
/// peek-through, C2) — as distinct from empty air a lower storey peeks through.
///
/// Real terrain is: a spawned terrain ENTITY (a per-def [`TerrainGraphicKey`] fact, i.e. an
/// authored wall / cover / slab / door / stair), a [`SlabState::Present`] surface, or a
/// non-[`TerrainKind::Open`] occupancy fact (a `Wall` / `Cover`). An [`Open`](TerrainKind::Open)
/// cell with no present slab and no spawned entity is EMPTY AIR on an upper storey — it emits
/// NOTHING, so the storey beneath peeks through (the floor-gap reveal). This is checked ONLY
/// for storeys above the ground floor; storey 0 always draws its full floor field (the ground
/// plane), matching the pre-GTW-519 single-storey behaviour.
pub(super) fn storey_has_terrain(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)>,
    map: &StaticMap,
) -> bool {
    facts.contains_key(key)
        || matches!(map.surface.slab_state(key), SlabState::Present)
        || !matches!(map.occupancy.terrain(key), TerrainKind::Open)
}

/// Converts a `usize` grid extent ([`GRID_WIDTH`](gdtf_battle_sim::occupancy::GRID_WIDTH) /
/// [`GRID_HEIGHT`](gdtf_battle_sim::occupancy::GRID_HEIGHT)) to the `i32` cell
/// coordinate range bound, saturating rather than wrapping.
///
/// [`GRID_WIDTH`](gdtf_battle_sim::occupancy::GRID_WIDTH) / [`GRID_HEIGHT`](gdtf_battle_sim::occupancy::GRID_HEIGHT)
/// are `usize` (flat-buffer extents); a cell coordinate
/// is `i32` ([`Cell`](gdtf_battle_sim::metric::Cell) wraps `IVec2`). `i32::try_from` clamps an
/// (impossible-in-practice) over-large extent to [`i32::MAX`] rather than wrap
/// (`cast_possible_wrap`).
pub(super) fn i32_extent(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
