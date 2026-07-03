//! The static-battlefield draw + cover-swap reaction, with the sim-grid bundle and the
//! presenter-owned sim-fact → tile-role mapping.

use std::ops::RangeInclusive;

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
    BattleReady, Cell, CellLevel, CoverDestroyed, CoverLedger, EmplacementState, FootfallSound,
    GRID_HEIGHT, GRID_WIDTH, Level, MAX_LEVELS, OccupancyGrid, SlabDestroyed, SlabState,
    SurfaceGrid, TerrainCell, TerrainGraphicKey, TerrainKind,
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    roles::{TileIndex, TileRole, TileRoles},
};
use crate::{Brightness, CELL_PX, SheetRole, TerrainFogMaterial, TopDownAtlases, cell_to_world};

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
    /// graphic-key resolution (the private `resolve_index` helper →
    /// [`TileRole::from_key`]) is tried FIRST; this role default applies only to a
    /// cell with no spawned terrain entity (the floor field) or an out-of-vocabulary
    /// graphic key.
    fn role_at(&self, key: &CellLevel) -> TileRole {
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
/// recipe), `saturation = 1.0` (seeded VISIBLE — the fog writer drives it per cell), and
/// `brightness = `[`Brightness::FULL`] (seeded full-bright — the fog writer dims it on a
/// lower drawn storey, GTW-519). Returns [`None`] if the terrain sheet OR its layout was not
/// loaded (so the caller skips the spawn rather than panic).
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
fn resolve_index(
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
fn storey_has_terrain(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)>,
    map: &StaticMap,
) -> bool {
    facts.contains_key(key)
        || matches!(map.surface.slab_state(key), SlabState::Present)
        || !matches!(map.occupancy.terrain(key), TerrainKind::Open)
}

/// Whether a `(cell, level)` key's storey lies within the drawn [`drawn_band`] (GTW-519 C6).
///
/// The shared predicate the two destroyed-swap reactions ([`swap_destroyed_cover`] /
/// [`swap_destroyed_slab`]) gate on so they act on any DRAWN storey (`0..=active`) and ignore
/// one strictly above the active view level. A [`CellLevel`]'s `z` is the `i32` storey index
/// ([`CellLevel`] wraps `IVec3`); the band's inclusive [`Level`] bounds read through
/// [`Level`]'s `Deref<Target = u8>` and compare against it. A negative or over-`u8` `z` (a
/// can't-happen malformed key) simply fails the bound rather than panicking.
fn cell_level_in_band(at: CellLevel, band: &RangeInclusive<Level>) -> bool {
    let start = i32::from(**band.start());
    let end = i32::from(**band.end());
    (start..=end).contains(&at.z)
}

/// Iterate the [`drawn_band`] as concrete [`Level`]s (BOTTOM-UP, `0..=active` inclusive).
///
/// A small adapter over the [`RangeInclusive<Level>`] the shared [`drawn_band`] returns:
/// [`Level`] wraps a `u8` but is not itself `Step` (no `Iterator` for the range), so this
/// walks the inclusive `u8` storey indices and re-wraps each through [`Level::new`], keeping
/// the loop bottom-up so painter's-Z occlusion holds by draw order + the per-storey z.
fn level_band(band: RangeInclusive<Level>) -> impl Iterator<Item = Level> {
    (**band.start()..=**band.end()).map(Level::new)
}

/// The inclusive band of storeys the terrain draw renders (GTW-519 / GTW-521), given the
/// current [`ActiveLevel`] and [`ViewMode`].
///
/// The UFO:EU / `OpenXcom` multi-level display: the band FLOOR is always the ground floor
/// (level 0 — the whole stack at/below the ceiling, not a windowed `[ceiling-N..=ceiling]`,
/// the GTW-519 logged fork (a)). The CEILING is the ONE thing the [`ViewMode`] chooses
/// (GTW-521):
///
/// - [`ViewMode::DownToActive`] (the default) → `0..=active`: draw up to and including the
///   active view level and CULL everything strictly above it — EXACTLY the GTW-519/520
///   behaviour, unchanged.
/// - [`ViewMode::FullView`] → `0..=MAX_LEVELS - 1`: draw the WHOLE storey stack regardless of
///   the active level (the UFO full-stack view). The upper roofs / floors re-appear and hide
///   the storeys / units beneath by per-storey Z; empty upper cells still emit nothing
///   (peek-through), so it stays cheap.
///
/// The painter's-algorithm occlusion falls out of the existing per-storey Z ([`z_for`], via
/// [`cell_to_world`]) either way — a higher storey's tile carries a strictly greater z, so it
/// draws in front, with NO new Z math.
///
/// The SINGLE readable definition of "which storeys are drawn", shared by the draw loop
/// (C1) AND the two destroyed-swap reactions (C6, [`swap_destroyed_cover`] /
/// [`swap_destroyed_slab`]) — and, through
/// [`ActiveLevel::draws_storey`](super::active_level::ActiveLevel), the GTW-520 ganger
/// visibility filter — so they can never drift. GTW-521's full-view toggle widens the band's
/// CEILING here in this ONE helper WITHOUT re-touching the loop body or the swap predicates.
///
/// [`z_for`]: crate::cell_to_world
pub(super) fn drawn_band(active: ActiveLevel, view: ViewMode) -> RangeInclusive<Level> {
    // The band floor is always the ground plane; only the ceiling depends on the view mode.
    let ceiling = match view {
        // DEFAULT (GTW-519/520): cull above the active view level.
        ViewMode::DownToActive => *active,
        // FULL VIEW (GTW-521): the top valid storey (`MAX_LEVELS - 1` = 7). `saturating_sub`
        // guards the impossible `MAX_LEVELS == 0`.
        ViewMode::FullView => Level::new(MAX_LEVELS.saturating_sub(1)),
    };
    Level::new(0)..=ceiling
}

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): the
/// static-battlefield ONE-SHOT draw + redraw-on-level-change.
///
/// Fires when ANY of: a [`BattleReady`](gdtf_battle_sim::BattleReady) drained this update,
/// [`ActiveLevel`] `is_changed()`, [`ViewMode`] `is_changed()` (GTW-521 — the full-view
/// toggle widens/narrows the [`drawn_band`] ceiling, so the terrain must redraw for the new
/// band), OR [`TileRoles`] `is_changed()` — the GTW-375 third
/// trigger that re-renders the terrain on a tile hot-reload. A `tile_roles.ron` re-save
/// MUTATES [`TileRoles`] (the indices swap) and an `alt_tileset_terrain.png` re-save
/// `set_changed()`s it (same indices, fresh GPU texture); either way the rendered tiles
/// SWAP, live, with no restart. It despawns ALL existing [`TerrainSprite`]
/// entities, then (GTW-519) for the whole DRAWN BAND [`drawn_band`] (`0..=active`,
/// BOTTOM-UP) scans `0..GRID_WIDTH` × `0..GRID_HEIGHT` per storey and spawns one terrain
/// tile per NON-EMPTY cell as a shared unit-rect [`Mesh2d`] +
/// [`MeshMaterial2d<TerrainFogMaterial>`] (GTW-348 — the material path so EXPLORED can render
/// greyscale; the `Sprite` pipeline cannot desaturate) at
/// [`cell_to_world`](crate::cell_to_world), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`TerrainSprite`] marker. The
/// ground floor (storey 0) draws its FULL floor field (the pre-GTW-519 single-storey
/// behaviour); every UPPER storey in the band draws ONLY cells with a real terrain fact
/// ([`storey_has_terrain`]) so an open/empty upper cell emits nothing and the storey beneath
/// PEEKS THROUGH (C2). Everything strictly ABOVE `active` is culled (the band never enters
/// it). The per-storey Z ([`z_for`](crate::cell_to_world) via [`cell_to_world`]) gives the
/// painter's-algorithm occlusion (a higher storey draws in front) with NO new Z math (C3).
/// The despawn-first step makes the first-ready double-fire (a `BattleReady` on the same
/// update `ActiveLevel` first reads `is_changed`) idempotent, and a level cycle redraws the
/// whole `[0..=active]` band (C7).
///
/// GTW-493 (T07c — the per-def presenter seam): each cell's atlas index is resolved from
/// the SIM-SPAWNED terrain entity's per-def [`TerrainGraphicKey`] FIRST (via the private
/// `resolve_index` helper → [`TileRole::from_key`]), falling back to the
/// [`TileRole`]-table default keyed on [`TerrainKind`] only for the floor field (no
/// spawned entity) or an out-of-vocabulary key (warned loudly — GTW-566 C4). So two `Cover` defs whose `graphic_name`s differ
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
    view: Res<ViewMode>,
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
    // GTW-521: a ViewMode toggle changes the drawn_band ceiling, so it must redraw the terrain
    // for the new band exactly as an ActiveLevel change does.
    if !ready_fired && !active.is_changed() && !view.is_changed() && !roles.is_changed() {
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

    // GTW-519: draw the whole storey band 0..=active BOTTOM-UP (the UFO:EU multi-level
    // display) rather than the single active storey — the band lives in ONE `drawn_band`
    // helper the swap reactions share (C1/C6). Iterating ascending keeps the loop reading
    // bottom-up; the painter's-algorithm occlusion is the existing per-storey Z (z_for via
    // cell_to_world), NOT new math — a higher storey's tile carries a strictly greater z, so
    // it draws in front. A storey strictly above active is never entered, so it emits nothing
    // (the hard cull). present_fog dims each lower storey's tiles per-tile afterward.
    for level in level_band(drawn_band(*active, *view)) {
        for y in 0..i32_extent(GRID_HEIGHT) {
            for x in 0..i32_extent(GRID_WIDTH) {
                let cell = Cell::new(x, y);
                let key = CellLevel::new(cell, level);
                // GTW-493: read the slab footfall from the def's presenter_kind for this
                // cell. It is Slab-ONLY and OPTIONAL: a Wall/Cover entity carries no
                // FootfallSound, and a Slab def may omit it. ABSENT footfall is handled here
                // with NO panic and a DOCUMENTED default — there is NO footfall-audio system
                // yet (the sim stays guns-only), so a present key is logged at `debug` (so a
                // future footfall pass has a wired seam to consume) and an absent one is the
                // SILENT default (no clip).
                if let Some((_graphic, Some(footfall))) = graphic_facts.get(&key) {
                    let footfall_key: &str = footfall;
                    debug!(
                        "terrain footfall present at {key:?}: `{footfall_key}` (no \
                         footfall-audio system yet — default: silent)",
                    );
                }
                // GTW-493: resolve the atlas index from this cell's per-def TerrainGraphicKey
                // FIRST (so two same-TerrainKind defs with distinct graphic_names draw
                // distinct sprites), falling back to the TerrainKind-keyed TileRole default
                // for the floor field / an out-of-vocabulary key.
                //
                // GTW-519 PEEK-THROUGH (C2): a `resolve_index` on a cell with nothing on it
                // still yields the FLOOR default — but the caller only spawns a tile where a
                // sim FACT exists on THIS storey. An open/empty upper-storey cell (no
                // terrain entity, no slab, occupancy `Open`) resolves to floor yet has no
                // fact keying it here for a non-zero storey, so it emits NOTHING and the
                // storey beneath peeks through. The floor field is authored on storey 0 (the
                // ground plane); upper storeys draw only their real walls / cover / slabs.
                if level != Level::new(0) && !storey_has_terrain(&key, &graphic_facts, &map) {
                    continue;
                }
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
                // (the sprite pipeline's per-channel multiply tint cannot desaturate).
                // Authored via `spawn_scene` (GTW-322): `Mesh2d` / `MeshMaterial2d` each wrap
                // a `Handle<_>`, which is NOT `Unpin` (so it has no `Template` impl) — exactly
                // like the old atlas `Sprite` — so each rides the `template(move |_|
                // Ok(value.clone()))` closure escape hatch (the `FnTemplate` output carries no
                // `Unpin` bound), the same one the AREA-1 widget builders use. The runtime
                // `Transform` / `RenderLayers` ARE `Clone + Default + Unpin`, so each rides
                // `template_value` (a value-overwrite). The `TerrainSprite` marker carries the
                // runtime source `CellLevel` (no `Default`), so it is `.insert`ed after the
                // scene. The tile is never read back by id (the redraw / cover-swap / fog find
                // it via the `TerrainSprite { at }` query, not a captured handle), so the
                // deferred materialization is inert — the same entity + components result.
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
}

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): swap
/// a destroyed cover cell's sprite to the RUBBLE tile.
///
/// Drains [`MessageReader<CoverDestroyed>`](gdtf_battle_sim::CoverDestroyed); for each
/// `CoverDestroyed { at }` WITHIN THE DRAWN BAND `[0..=active]` (GTW-519 C6 — the shared
/// [`drawn_band`] predicate, so a cover smashed on any drawn lower storey swaps too) it finds
/// the [`TerrainSprite`] at `at` and swaps its texture-atlas index to the `rubble`
/// [`TileIndex`] (read from [`TileRoles`], never a literal). A destruction on a storey
/// strictly ABOVE the active view level is ignored (that terrain is not drawn). Choice: SWAP
/// (not despawn) so the cell still reads as terrain (rubble) rather than a hole — AC3 asserts
/// the swap.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<TileRoles>`],
/// [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap re-indexes the tile's
/// material rather than its sprite), [`MessageReader<CoverDestroyed>`], and the
/// [`TerrainSprite`] / [`MeshMaterial2d`] query (to find the material at `at`) — no
/// [`Commands`] needed, the swap edits the material in place (no despawn / respawn).
pub fn swap_destroyed_cover(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<CoverDestroyed>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let band = drawn_band(*active, *view);
    let rubble = *roles.rubble;
    for event in destroyed.read() {
        // CoverDestroyed.at is a CellLevel; only act on cells WITHIN the drawn band
        // [0..=active] (GTW-519 C6 — a cover smashed on a DRAWN lower storey still swaps to
        // rubble; one strictly ABOVE the active view level is not drawn, so there is no tile
        // to re-index). SHARES the `drawn_band` helper with the draw loop so the two cannot
        // drift.
        if !cell_level_in_band(event.at, &band) {
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
/// `SlabDestroyed { at }` WITHIN THE DRAWN BAND `[0..=active]` (GTW-519 C6 — the shared
/// [`drawn_band`] predicate) it finds the [`TerrainSprite`] at `at` and swaps its
/// texture-atlas index to the `slab_destroyed` [`TileIndex`] (read from [`TileRoles`], never
/// a literal — the engineer's-choice destroyed-slab treatment, which the
/// [`TileRoles::slab_destroyed`] doc-comment describes and flags for art review). A
/// destruction on a storey strictly ABOVE the active view level is ignored (that terrain is
/// not drawn). Choice: SWAP (not despawn) so the cell still reads as terrain (rubble/debris)
/// rather than a hole, and so the SAME sprite `Entity` persists across the swap (the UI
/// mutate-not-respawn rule, C7) — exactly the cover path.
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
    view: Res<ViewMode>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<SlabDestroyed>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let band = drawn_band(*active, *view);
    let slab_destroyed = *roles.slab_destroyed;
    for event in destroyed.read() {
        // SlabDestroyed.at is a CellLevel; only act on cells WITHIN the drawn band
        // [0..=active] (GTW-519 C6 — a slab smashed on a DRAWN lower storey swaps to its
        // destroyed tile; one strictly ABOVE the active view level is not drawn, so nothing
        // to re-index there). SHARES the `drawn_band` helper with the draw loop.
        if !cell_level_in_band(event.at, &band) {
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

/// `Update` (`PresenterSystems::Draw`, gated `resource_exists::<BattleInProgress>`): swap a
/// weapon-emplacement's tile between its VACANT and OCCUPIED sprite as its
/// [`EmplacementState`](gdtf_battle_sim::EmplacementState) changes (GTW-543 — the
/// occupied-state visual indicator).
///
/// The state analogue of [`swap_destroyed_cover`] / [`swap_destroyed_slab`]: rather than a
/// one-shot destruction MESSAGE, it reacts to the sim's per-entity
/// [`Changed<EmplacementState>`](gdtf_battle_sim::EmplacementState) — the enter/exit toggle
/// (`apply_emplacement_toggle`) flips the emplacement entity's state Vacant↔Occupied, and this
/// system mirrors that onto the drawn tile. For each emplacement whose state CHANGED and whose
/// cell lies WITHIN THE DRAWN BAND `[0..=active]` (the shared `drawn_band` predicate, so an
/// emplacement manned on any drawn lower storey re-tints too; one strictly ABOVE the active view
/// level is not drawn, so there is no tile to re-index), it finds the [`TerrainSprite`] at that
/// cell and re-indexes its material to the `emplacement_occupied` [`TileIndex`] while
/// [`Occupied`](gdtf_battle_sim::EmplacementState::Occupied), or back to the `emplacement` tile
/// while [`Vacant`](gdtf_battle_sim::EmplacementState::Vacant) — both read from [`TileRoles`],
/// never a literal.
///
/// Choice: SWAP (not despawn / overlay) so the SAME sprite `Entity` persists across the state
/// change (the UI mutate-not-respawn rule, mirroring the two destruction swaps) and the cell
/// keeps reading as terrain. `Changed` fires on the FIRST observation too (the freshly-spawned
/// `Vacant` emplacement), which is inert — it re-indexes the already-`emplacement` tile to the
/// `emplacement` index (a no-op), so a battle-start pass never mis-tints an unmanned mount.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<ViewMode>`], [`Res<TileRoles>`],
/// [`ResMut<Assets<TerrainFogMaterial>>`] (the swap re-indexes the tile's material in place — the
/// destruction-swap precedent), the `Changed<EmplacementState>` sim query (the emplacement's
/// state + its [`TerrainCell`](gdtf_battle_sim::TerrainCell)), and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] query (to find the material at the emplacement's cell). No [`Commands`]
/// needed — the swap edits the material, never spawning / despawning.
pub fn indicate_emplacement_occupied(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    emplacements: Query<(&EmplacementState, &TerrainCell), Changed<EmplacementState>>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let band = drawn_band(*active, *view);
    let vacant_index = *roles.emplacement;
    let occupied_index = *roles.emplacement_occupied;
    for (state, cell) in &emplacements {
        // TerrainCell derefs to the emplacement's CellLevel. Only act on cells WITHIN the drawn
        // band [0..=active] (the drawn_band predicate the destruction swaps share) — an
        // emplacement on a storey strictly above the active view level is not drawn.
        let at = **cell;
        if !cell_level_in_band(at, &band) {
            continue;
        }
        // The tile the manned/unmanned state resolves to: the OCCUPIED variant while manned,
        // the VACANT emplacement tile otherwise (a Vacant `Changed` first-observation re-indexes
        // to the same index it already carries — an inert no-op, never a mis-tint).
        let target = if state.is_occupied() {
            occupied_index
        } else {
            vacant_index
        };
        for (terrain, mat_handle) in &tiles {
            if terrain.at != at {
                continue;
            }
            // Re-index the tile's material in place — keep the entity (the mutate-not-respawn
            // rule). get_mut marks the material dirty so the UV-transform uniform re-uploads next
            // frame (mirroring the destruction swaps). A tile whose material was dropped (the
            // terrain sheet was absent at draw time) is simply not in the query; nothing to
            // re-index.
            if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                material.atlas_index = target;
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
