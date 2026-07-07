//! The sim-fact read model: the [`StaticMap`] borrow-bundle, the graphic-name /
//! sprite-def resolution bundle ([`SpriteResolveCtx`]), and the grid-extent util.

use bevy::{ecs::system::SystemParam, image::Image, platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{
    cover::CoverLedger,
    entity::TerrainCell,
    occupancy::TerrainKind,
    piece::{FootfallSound, TerrainGraphicKey},
    prelude::{CellLevel, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::{
    resolve::{
        MissingTileTexture, anchor_world_offset, resolve_sprite, single_rect_layout, source_parts,
        source_px_size, source_urect,
    },
    roles::TileRole,
};
use crate::{Brightness, CELL_PX, TerrainFogMaterial};

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
    /// mapping is the one place the fallback CHOICE lives; the PIXELS a role resolves to
    /// are the role key's sprite def via
    /// [`resolve_sprite`](super::resolve::resolve_sprite) — GTW-665, never a hardcoded
    /// index): a `Present` slab is [`TileRole::Slab`]; a [`TerrainKind::Wall`] is
    /// [`TileRole::Wall`]; a [`TerrainKind::Cover`] (corroborated by a [`CoverLedger`]
    /// `peek`, NEVER a seeding accessor) is [`TileRole::Cover`]; an open cell with no
    /// present slab is the [`TileRole::Floor`] default. Every in-range cell is at least
    /// floor, so this always yields a role — the caller scans only in-range cells, so the
    /// out-of-range `Open` default of [`OccupancyGrid::terrain`] (FLOOR) never spawns a
    /// stray off-grid sprite.
    ///
    /// This is the FALLBACK keyed only on [`TerrainKind`] — the GTW-493 per-def
    /// graphic-key resolution ([`graphic_name_at`]) is tried FIRST; this
    /// role default applies only to a cell with no spawned terrain entity (the floor
    /// field).
    pub(super) fn role_at(&self, key: &CellLevel) -> TileRole {
        // A present slab tile sits on the active level (slab_state is CellLevel-keyed).
        if matches!(self.surface.slab_state(key), SlabState::Present) {
            return TileRole::Slab;
        }
        match self.occupancy.terrain(key) {
            TerrainKind::Wall => TileRole::Wall,
            // GTW-543: an emplacement is a cover-like smashable structure — it falls back to
            // the Cover role here (the per-def graphic-key resolution via GTW-493
            // `graphic_name_at` fires FIRST and picks the authored emplacement sprite by
            // its `graphic_name`, so this role default only applies to a cell with no spawned
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

/// The graphic NAME an in-range cell draws (GTW-493 / GTW-665): the sim-spawned per-def
/// [`TerrainGraphicKey`] FIRST (so two same-[`TerrainKind`] defs with distinct
/// `graphic_name`s draw distinct sprites), else the [`StaticMap::role_at`] role KEY
/// (the floor field / an entity-less occupancy fact). Either way the name then resolves
/// through the ONE def resolution ([`SpriteResolveCtx::resolved`]).
pub(super) fn graphic_name_at<'a>(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&'a TerrainGraphicKey, Option<&FootfallSound>)>,
    map: &StaticMap,
) -> &'a str {
    match facts.get(key) {
        Some((graphic, _footfall)) => graphic,
        None => map.role_at(key).as_key(),
    }
}

/// The def-resolution borrow bundle (GTW-665) — everything a terrain draw / swap needs
/// to turn a graphic NAME into pixels: the [`SpriteDefRegistry`] (the resolution
/// source), the [`AssetServer`] (a def's named source image — the SAME handle for the
/// same path, so the seeded defs sample the identical sheet texture), the loaded
/// [`Assets<Image>`] (a `File` source's decoded dims for the anchor), and the
/// [`MissingTileTexture`] (the C4 magenta marker).
///
/// A `#[derive(SystemParam)]` bundle (the [`StaticMap`] shape) so the draw + the three
/// swap reactions stay under the `too_many_arguments` gate while sharing literally one
/// resolution body.
#[derive(SystemParam)]
pub struct SpriteResolveCtx<'w> {
    /// The GTW-663 sprite-def catalog the names resolve against.
    defs:         Res<'w, SpriteDefRegistry>,
    /// Loads a def's source image by its authored path.
    asset_server: Res<'w, AssetServer>,
    /// The decoded images — a `File` source's dims for the anchor offset.
    images:       Res<'w, Assets<Image>>,
    /// The loud magenta missing-sprite marker (GTW-665 C4).
    missing:      Res<'w, MissingTileTexture>,
}

impl SpriteResolveCtx<'_> {
    /// Whether the sprite-def registry changed this tick — the GTW-666 restamp's
    /// trigger (a `.spritedef.ron` re-save rebuilds the registry through the family
    /// redrive; [`restamp_tiles_on_def_change`](super::restamp::restamp_tiles_on_def_change)
    /// then re-resolves every already-drawn tile in place).
    pub(super) fn defs_changed(&self) -> bool {
        self.defs.is_changed()
    }

    /// Resolve a graphic `name` to its drawn (material, anchor offset) — THE GTW-665
    /// resolution, shared by the one-shot draw and the in-place swap reactions.
    ///
    /// `def.source` picks the texture: a `Sheet` source loads the sheet image by its
    /// authored path and carries the authored rect as a
    /// [`single_rect_layout`] at index `0` (byte-identical UV math to the retired grid
    /// layout for a grid-aligned rect — the identical-pixels claim); a `File` source
    /// loads the standalone image with NO layout (identity UV). `def.anchor` drives
    /// placement (C2): the returned offset displaces the sprite's CENTER so the authored
    /// ground-contact/pivot sits ON the cell position — `Vec2::ZERO` for the seeded
    /// center anchors. The sprite's pixel extent layers two knowledge sources (the
    /// GTW-664 precedent): a `Sheet` rect's `w × h` cheaply, a `File`'s LOADED dims once
    /// decoded, else the centered default (documented, no panic).
    ///
    /// A name resolving NO def (possible mid-authoring — the integrity edge warns, load
    /// still exits) WARNS, naming the name and the cell, and returns the LOUD magenta
    /// [`MissingTileTexture`] marker at the centered default (C4): never a panic, never
    /// an invisible tile.
    pub(super) fn resolved(&self, name: &str, at: &CellLevel) -> (TerrainFogMaterial, Vec2) {
        let (image, region, offset) = self.resolved_parts(name, at);
        let material = TerrainFogMaterial {
            image,
            atlas_layout: region.map(single_rect_layout),
            atlas_index: 0,
            custom_size: Some(Vec2::splat(CELL_PX)),
            // Seed VISIBLE (full colour); present_fog drives it to 0.0 on EXPLORED cells.
            saturation: 1.0,
            // GTW-519: seed full brightness; present_fog dims a lower drawn storey per-tile.
            brightness: Brightness::FULL,
        };
        (material, offset)
    }

    /// The `Sprite`-path projection of [`resolved`](Self::resolved) — the SAME
    /// resolution rendered as an atlas-free [`Sprite`] (image + pixel `rect` +
    /// `custom_size`) for the vertical-link tiles (GTW-665): the sprite pipeline reads
    /// `Sprite::rect` exactly as it read a `TextureAtlas` layout entry, so a
    /// grid-aligned seeded rect draws the identical pixels. Same C2 anchor offset,
    /// same LOUD C4 missing-marker fallback.
    pub(super) fn resolved_sprite(&self, name: &str, at: &CellLevel) -> (Sprite, Vec2) {
        let (image, region, offset) = self.resolved_parts(name, at);
        let mut sprite = Sprite::from_image(image);
        sprite.rect = region.map(|region| region.as_rect());
        sprite.custom_size = Some(Vec2::splat(CELL_PX));
        (sprite, offset)
    }

    /// The shared resolution body: `name` → def → (source image handle, OPTIONAL pixel
    /// region, C2 anchor offset) — or the magenta marker triple (whole image, zero
    /// offset) with a LOUD `warn!` when no def carries the name (C4).
    fn resolved_parts(&self, name: &str, at: &CellLevel) -> (Handle<Image>, Option<URect>, Vec2) {
        let Some(def) = resolve_sprite(&self.defs, name) else {
            warn!(
                "terrain draw: no sprite def named `{name}` at {at:?} — drawing the magenta \
                 missing-sprite marker instead",
            );
            return (self.missing.handle(), None, Vec2::ZERO);
        };
        let (path, rect) = source_parts(&def.source);
        let image = self.asset_server.load(path.as_str().to_owned());
        let px = source_px_size(&def.source).or_else(|| self.images.get(&image).map(Image::size));
        let offset = px.map_or(Vec2::ZERO, |px| {
            anchor_world_offset(def, px, Vec2::splat(CELL_PX))
        });
        (image, rect.map(source_urect), offset)
    }
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
