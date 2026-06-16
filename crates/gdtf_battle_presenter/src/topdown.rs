//! Top-down renderer plumbing: the px/coordinate bridge the 16×16 top-down sprite
//! renderer is built on.
//!
//! This module owns the presenter-side projection OF the sim's presentation-agnostic
//! cubic-voxel metric (`docs/combat/battle-space.md`): the sim reasons in cells +
//! levels, and turning those into on-screen world units is the presenter's job
//! (ADR-0001 — the presenter owns ALL sim→view projection; the sim never reads the
//! presenter). It defines [`CELL_PX`], the [`cell_to_world`] projection, the
//! role-keyed atlas [`TopDownAtlases`] resource, and the system
//! ([`load_topdown_atlases`]) that builds that resource ONCE from the three render
//! sheets (terrain / characters / effects).
//!
//! It spawns NO sprite and draws NOTHING — terrain is S4, characters S5, FX S6. It
//! only produces the loaded atlas resource, the [`CELL_PX`] scalar, the
//! [`cell_to_world`] projection, and the documented sprite-sizing recipe those later
//! slices call.
//!
//! # Sprite-construction recipe (for S4/S5/S6)
//!
//! Every cell sprite is sized to exactly one cell so a 16px source tile fills a
//! 16-world-unit cell regardless of camera scale:
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy::image::TextureAtlas;
//! use gdtf_battle_presenter::topdown::{CELL_PX, SheetRole, TopDownAtlases};
//!
//! fn draw_one(mut commands: Commands, atlases: Res<TopDownAtlases>) {
//!     let Some(role) = atlases.role(SheetRole::Terrain) else { return };
//!     let mut sprite = Sprite::from_atlas_image(
//!         role.image.clone(),
//!         TextureAtlas { layout: role.layout.clone(), index: 0 },
//!     );
//!     sprite.custom_size = Some(Vec2::splat(CELL_PX));
//!     commands.spawn(sprite);
//! }
//! ```
//!
//! Equivalently, the explicit `Sprite { image, texture_atlas: Some(TextureAtlas {
//! layout, index }), custom_size: Some(Vec2::splat(CELL_PX)), .. }` form. Any
//! tile-INDEX value a later slice introduces must be a NAMED newtype over its
//! primitive (no-bare-types); S3 itself stores no per-glyph index.

use bevy::{image::TextureAtlasLayout, platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{Cell, Level};

/// On-screen size of one cell, in world units.
///
/// The ONE presenter source of truth for cell size. A `const`, NOT a domain newtype
/// — the framework-plumbing carve-out (`.claude/rules/no-bare-types.md` clause 4):
/// a scalar fed straight to a [`Transform`] / `custom_size`, not a domain quantity,
/// the same reasoning the landed `WORLD_RENDER_LAYER`-class consts use. 16.0 because
/// the source tiles are 16×16 px, so one source tile maps to a 16-world-unit cell.
pub const CELL_PX: f32 = 16.0;

/// Per-level world-space draw-z spacing, in world units.
///
/// A small monotonic gap between storeys so sprites on different levels do not
/// z-fight. Full multi-level z-stacking is GTW-49 / GTW-10; this slice only needs a
/// stable per-level z for the projection.
const Z_PER_LEVEL: f32 = 1.0;

/// Projects a sim cell + level into the top-down renderer's world-space position.
///
/// Row 0 sits at the TOP: Bevy's +Y is up, so a larger `cell.y` (further down the
/// grid) yields a smaller world `y`. `x` grows right by exactly [`CELL_PX`] per cell.
/// The `z` is a stable per-level draw-z ([`z_for`]) so sprites on different storeys
/// do not z-fight; it is NOT full multi-level stacking (GTW-49 / GTW-10).
///
/// `cell.x` / `cell.y` read through [`Cell`]'s `Deref<Target = IVec2>`; the level
/// index reads through [`Level`]'s `Deref<Target = u8>` inside [`z_for`].
#[must_use]
pub fn cell_to_world(cell: Cell, level: Level) -> Vec3 {
    Vec3::new(
        cell.x as f32 * CELL_PX,
        -(cell.y as f32) * CELL_PX,
        z_for(level),
    )
}

/// The world-space draw-z for a storey `level`.
///
/// Monotonic in the storey index so higher storeys draw in front: `*level` (read
/// through [`Level`]'s `Deref<Target = u8>`) scaled by [`Z_PER_LEVEL`]. Kept private
/// — callers use [`cell_to_world`].
fn z_for(level: Level) -> f32 {
    f32::from(*level) * Z_PER_LEVEL
}

/// Which of the role-separated 16×16 sprite sheets an atlas entry belongs to.
///
/// A domain value (a real named type, not a bare string key), per
/// `.claude/rules/no-bare-types.md`. This slice loads the three render sheets the
/// playable slices consume; the deferred `ui` / `items` / `portraits` sheets join
/// this enum in their consuming slices (GTW-222 / later) via the same mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetRole {
    /// Terrain tiles — `tiles/alt_tileset_terrain.png` (the S4 draw sheet).
    Terrain,
    /// Character tiles — `tiles/alt_tileset_characters.png` (the S5 draw sheet).
    Characters,
    /// Effect tiles — `tiles/alt_tileset_effects.png` (the S6 draw sheet).
    Effects,
}

impl SheetRole {
    /// The three render sheets this slice loads, in a fixed order.
    ///
    /// `ui` / `items` / `portraits` are deferred to their consuming slices and are
    /// deliberately absent here.
    const ALL: [Self; 3] = [Self::Terrain, Self::Characters, Self::Effects];

    /// Loose-file path (relative to the asset source root) of this sheet's PNG.
    ///
    /// The path a working-dir-at-workspace-root app and a workspace-rooted test
    /// both resolve to the shipped sheet.
    const fn asset_path(self) -> &'static str {
        match self {
            Self::Terrain => "tiles/alt_tileset_terrain.png",
            Self::Characters => "tiles/alt_tileset_characters.png",
            Self::Effects => "tiles/alt_tileset_effects.png",
        }
    }

    /// This sheet's grid shape as `(columns, rows)` of 16×16 tiles.
    ///
    /// terrain 16×22 (352 tiles), characters 16×18 (288), effects 16×8 (128) — the
    /// `from_grid` dimensions for [`load_topdown_atlases`].
    const fn grid(self) -> (u32, u32) {
        match self {
            Self::Terrain => (16, 22),
            Self::Characters => (16, 18),
            Self::Effects => (16, 8),
        }
    }
}

/// One render sheet's loaded handles: its image plus the atlas layout over it.
///
/// A NAMED type, per `.claude/rules/no-bare-types.md` — no bare `Handle<...>` stored
/// as a domain value. The S4/S5/S6 draw systems read these to build sprites (see the
/// module-level sprite recipe).
#[derive(Debug, Clone)]
pub struct SheetAtlas {
    /// The sheet image handle (loaded via [`AssetServer::load`]).
    pub image:  Handle<Image>,
    /// The grid layout over [`Self::image`], one entry per 16×16 tile.
    pub layout: Handle<TextureAtlasLayout>,
}

/// The presenter-owned, role-keyed atlas resource.
///
/// Holds, KEYED BY [`SheetRole`], the loaded [`SheetAtlas`] (image + layout handles)
/// for each render sheet. Built ONCE by [`load_topdown_atlases`] and present before
/// the S4/S5/S6 draw systems run, which read it to spawn sprites. A framework type
/// (`Resource`), exempt from no-bare-types; the role key it stores is the named
/// [`SheetRole`]. The deferred `ui` / `items` / `portraits` sheets (GTW-222 / later)
/// join the same map via the same mechanism.
#[derive(Resource, Debug, Clone)]
pub struct TopDownAtlases {
    /// One loaded [`SheetAtlas`] per loaded [`SheetRole`].
    sheets: HashMap<SheetRole, SheetAtlas>,
}

impl TopDownAtlases {
    /// The loaded [`SheetAtlas`] for `role`, if that role was loaded.
    ///
    /// Returns [`None`] for a role not in this slice's loaded set (e.g. a deferred
    /// `ui` / `items` / `portraits` sheet) — callers match the [`Option`] rather
    /// than risk a panic.
    #[must_use]
    pub fn role(&self, role: SheetRole) -> Option<&SheetAtlas> {
        self.sheets.get(&role)
    }
}

/// Loads the three render sheets and inserts the [`TopDownAtlases`] resource.
///
/// Runs ONCE (the [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) schedules
/// it in [`Startup`]): for each [`SheetRole`] it loads the PNG via
/// [`AssetServer::load`] (NOT the RON loader — that is `.ron`-only) and builds one
/// [`TextureAtlasLayout::from_grid`] per sheet (16×16 tiles, the role's `(cols,
/// rows)`), adding each layout to [`Assets<TextureAtlasLayout>`]. The resource is
/// inserted via [`Commands`] so it is present before the draw slices run.
///
/// Per `.claude/rules/bevy-traps.md` #7 this takes only normal system params — no
/// `&mut World`: [`Commands`] for the resource insert, [`Res<AssetServer>`] for the
/// image loads, and [`ResMut<Assets<TextureAtlasLayout>>`] to register the layouts.
pub fn load_topdown_atlases(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut sheets = HashMap::default();
    for role in SheetRole::ALL {
        let (columns, rows) = role.grid();
        let layout = TextureAtlasLayout::from_grid(UVec2::splat(16), columns, rows, None, None);
        sheets.insert(
            role,
            SheetAtlas {
                image:  asset_server.load(role.asset_path()),
                layout: layouts.add(layout),
            },
        );
    }

    commands.insert_resource(TopDownAtlases { sheets });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC2 — `CELL_PX` is exactly 16.0, the single source of truth for cell size.
    #[test]
    fn cell_px_is_sixteen() {
        assert_eq!(
            CELL_PX.to_bits(),
            16.0_f32.to_bits(),
            "CELL_PX must be exactly 16.0 (one 16px source tile per cell)",
        );
    }

    /// AC3 — `cell_to_world` puts row 0 at the top (Bevy +Y up) and grows `x` with
    /// `cell.x` by exactly `CELL_PX`; `z` is the per-level draw-z.
    #[test]
    fn cell_to_world_projects_row_zero_to_the_top() {
        let l0 = Level::new(0);

        // Origin maps to x == 0.
        assert_eq!(
            cell_to_world(Cell::new(0, 0), l0).x.to_bits(),
            0.0_f32.to_bits(),
            "cell (0,0) must project to world x == 0",
        );

        // Greater cell.y goes DOWN the grid => smaller (more negative) world y,
        // so row 0 sits ABOVE row 1.
        assert!(
            cell_to_world(Cell::new(0, 0), l0).y >= cell_to_world(Cell::new(0, 1), l0).y,
            "row 0 must be at least as near the top (greater y) as row 1",
        );

        // x grows by exactly CELL_PX per column; y is -CELL_PX per row.
        let p = cell_to_world(Cell::new(1, 2), l0);
        assert_eq!(
            p.x.to_bits(),
            (1.0 * CELL_PX).to_bits(),
            "cell.x == 1 must project to x == 1 * CELL_PX",
        );
        assert_eq!(
            p.y.to_bits(),
            (-2.0 * CELL_PX).to_bits(),
            "cell.y == 2 must project to y == -2 * CELL_PX",
        );

        // z is the per-level draw-z for L0.
        assert_eq!(p.z.to_bits(), z_for(l0).to_bits(), "z must equal z_for(L0)");
    }

    /// `z_for` is monotonic in the storey index so higher storeys draw in front.
    #[test]
    fn z_for_is_monotonic_in_storey() {
        assert!(
            z_for(Level::new(1)) > z_for(Level::new(0)),
            "a higher storey must draw at a greater z",
        );
    }
}
