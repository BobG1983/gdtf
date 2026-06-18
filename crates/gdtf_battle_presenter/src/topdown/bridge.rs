//! The px/coordinate bridge definitions: [`CELL_PX`], the [`cell_to_world`] projection,
//! the role-keyed atlas resource, and the atlas-load system.

use bevy::{
    image::{ImageLoaderSettings, ImageSampler, TextureAtlasLayout},
    platform::collections::HashMap,
    prelude::*,
};
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

/// The within-storey draw-z bias that lifts a ganger sprite ABOVE its own floor tile.
///
/// A `const`, NOT a domain newtype — the `CELL_PX`-class framework-plumbing carve-out
/// (`.claude/rules/no-bare-types.md` clause 4): a scalar fed straight to a
/// [`Transform`]'s `z`, not a domain quantity. Strictly `< Z_PER_LEVEL` (`0.1 < 1.0`)
/// so a ganger's lifted z never sorts into the NEXT storey's band — it stays within its
/// own storey, just in front of the same-cell terrain (which draws at the bare level z,
/// [`Layer::Terrain`] = `0.0`). This fixes the GTW-283 occlusion: at storey 0 the ganger
/// and its floor both projected to `z = 0.0`, and Bevy 0.18's non-deterministic same-z 2D
/// sort let the opaque floor draw over the ganger. `0.1` is the smallest legible lift.
pub const GANGER_Z_BIAS: f32 = 0.1;

/// A presenter draw layer within a single storey — the ONE place the
/// terrain &lt; actor &lt; highlight stacking order lives.
///
/// A domain value (a real named type, not a bare z magnitude), per
/// `.claude/rules/no-bare-types.md`. Each layer's [`z_bias`](Layer::z_bias) is added on
/// top of the per-storey level z by [`cell_to_world_layered`] so a sprite draws in front
/// of the lower layers at its own cell without crossing into the next storey's band (every
/// bias is strictly `< Z_PER_LEVEL`). This slice wires [`Terrain`](Layer::Terrain) (the
/// bare level z, via [`cell_to_world`]) and [`Actor`](Layer::Actor) (the
/// [`GANGER_Z_BIAS`] lift); [`Highlight`](Layer::Highlight) is defined so the documented
/// order is complete, but routing the hover/selection highlight through it is the
/// in-engine-adjustable later tweak the GTW-283 contract flags (it currently still draws
/// at the bare level z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Floor / wall / cover terrain — the ground plane, drawn at the bare per-storey z.
    Terrain,
    /// A ganger (actor) — drawn just in front of its own terrain by [`GANGER_Z_BIAS`].
    Actor,
    /// The hover / selection highlight — drawn in front of the actor so it tints the unit
    /// (documented order; wiring deferred, see the type doc).
    Highlight,
}

impl Layer {
    /// This layer's within-storey draw-z bias, added on top of the per-storey level z.
    ///
    /// Strictly increasing terrain &lt; actor &lt; highlight, and every value is strictly
    /// `< Z_PER_LEVEL` so a biased sprite never sorts into the next storey's band.
    #[must_use]
    const fn z_bias(self) -> f32 {
        match self {
            Self::Terrain => 0.0,
            Self::Actor => GANGER_Z_BIAS,
            // Strictly above the actor, still within the storey band (`< Z_PER_LEVEL`).
            Self::Highlight => GANGER_Z_BIAS * 2.0,
        }
    }
}

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

/// Projects a sim cell + level into world-space, lifted by `layer`'s within-storey
/// draw-z bias — the [`cell_to_world`] position with [`Layer::z_bias`] added to `z`.
///
/// The ONE place a presenter draws a sprite "in front of" the lower layers at the same
/// cell: [`Terrain`](Layer::Terrain) sits at the bare per-storey z (equivalent to
/// [`cell_to_world`]), [`Actor`](Layer::Actor) is lifted by [`GANGER_Z_BIAS`] so a ganger
/// draws over its own floor tile (GTW-283), and the lift never crosses into the next
/// storey (every bias is strictly `< Z_PER_LEVEL`). `x` / `y` are unchanged from
/// [`cell_to_world`].
#[must_use]
pub fn cell_to_world_layered(cell: Cell, level: Level, layer: Layer) -> Vec3 {
    let mut world = cell_to_world(cell, level);
    world.z += layer.z_bias();
    world
}

/// The world-space draw-z for a storey `level`.
///
/// Monotonic in the storey index so higher storeys draw in front: `*level` (read
/// through [`Level`]'s `Deref<Target = u8>`) scaled by [`Z_PER_LEVEL`]. Kept private
/// — callers use [`cell_to_world`].
pub(super) fn z_for(level: Level) -> f32 {
    f32::from(*level) * Z_PER_LEVEL
}

/// Which of the role-separated sprite sheets an atlas entry belongs to.
///
/// A domain value (a real named type, not a bare string key), per
/// `.claude/rules/no-bare-types.md`. This loads the three 16×16 render sheets the
/// playable slices consume PLUS the GTW-278 [`Portraits`](SheetRole::Portraits) face
/// sheet (32×32 cells) the status / hover panels read; the remaining deferred `ui` /
/// `items` sheets join this enum in their consuming slices via the same mechanism. The
/// per-sheet tile size is NOT hardcoded — each role declares its own [`tile_px`](SheetRole::tile_px)
/// (the render sheets stay 16, the portrait sheet is 32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetRole {
    /// Terrain tiles — `tiles/alt_tileset_terrain.png` (the S4 draw sheet).
    Terrain,
    /// Character tiles — `tiles/alt_tileset_characters.png` (the S5 draw sheet).
    Characters,
    /// Effect tiles — `tiles/alt_tileset_effects.png` (the S6 draw sheet).
    Effects,
    /// Portrait faces — `tiles/alt_tileset_portraits.png` (the GTW-278 HUD sheet): a
    /// 10×10 grid of 32×32-px faces, indices `0..=99`. Read as a `bevy_ui` `ImageNode`
    /// atlas variant by the status / hover panels' shared stat block, NOT a world
    /// sprite (the UI layer, not the map). Unlike the render sheets its cells are 32 px
    /// ([`tile_px`](SheetRole::tile_px)).
    Portraits,
}

impl SheetRole {
    /// Every sheet this loader loads, in a fixed order.
    ///
    /// The three 16×16 render sheets plus the GTW-278 32×32 portrait sheet; the
    /// deferred `ui` / `items` sheets are still absent here.
    const ALL: [Self; 4] = [
        Self::Terrain,
        Self::Characters,
        Self::Effects,
        Self::Portraits,
    ];

    /// Loose-file path (relative to the asset source root) of this sheet's PNG.
    ///
    /// The path a working-dir-at-workspace-root app and a workspace-rooted test
    /// both resolve to the shipped sheet.
    const fn asset_path(self) -> &'static str {
        match self {
            Self::Terrain => "tiles/alt_tileset_terrain.png",
            Self::Characters => "tiles/alt_tileset_characters.png",
            Self::Effects => "tiles/alt_tileset_effects.png",
            Self::Portraits => "tiles/alt_tileset_portraits.png",
        }
    }

    /// This sheet's grid shape as `(columns, rows)` of [`tile_px`](SheetRole::tile_px)
    /// cells.
    ///
    /// terrain 16×22 (352 tiles), characters 16×18 (288), effects 16×8 (128) — all of
    /// 16×16 px — and the GTW-278 portraits 10×10 (100 faces) of 32×32 px; the
    /// `from_grid` dimensions for [`load_topdown_atlases`]. `pub(super)` so the sibling
    /// `topdown::test` module can pin the per-sheet grid without an app harness.
    pub(super) const fn grid(self) -> (u32, u32) {
        match self {
            Self::Terrain => (16, 22),
            Self::Characters => (16, 18),
            Self::Effects => (16, 8),
            Self::Portraits => (10, 10),
        }
    }

    /// This sheet's per-cell tile size, in source pixels — the square edge of one
    /// atlas cell.
    ///
    /// The render sheets are 16-px tiles; the GTW-278 portrait sheet is 32-px faces.
    /// Threaded into [`TextureAtlasLayout::from_grid`] per sheet by
    /// [`load_topdown_atlases`] so the portrait layout is NOT mis-sized to 16 (which
    /// would carve each 32-px face into four wrong sub-tiles). A `const`, NOT a domain
    /// newtype — the `CELL_PX`-class framework-plumbing carve-out
    /// (`.claude/rules/no-bare-types.md` clause 4): it is a layout dimension fed
    /// straight to `from_grid`, not a domain quantity. `pub(super)` so the sibling
    /// `topdown::test` module can pin the per-sheet tile size without an app harness.
    pub(super) const fn tile_px(self) -> u32 {
        match self {
            Self::Terrain | Self::Characters | Self::Effects => 16,
            Self::Portraits => 32,
        }
    }

    /// This sheet's per-asset texture SAMPLER override, or [`None`] to keep the asset
    /// server's default (GTW-295).
    ///
    /// The render sheets ([`Terrain`](SheetRole::Terrain) /
    /// [`Characters`](SheetRole::Characters) / [`Effects`](SheetRole::Effects)) draw as world
    /// sprites at ~1:1 source-to-screen and do not bleed, so they keep the default sampler
    /// ([`None`]). The [`Portraits`](SheetRole::Portraits) sheet is upscaled into a `bevy_ui`
    /// portrait node much larger than its 32-px faces; the default LINEAR sampler bilinearly
    /// blends the transparent-white (255,255,255,0) rows inset at the top of each face into a
    /// whitish fringe — so it overrides to [`ImageSampler::nearest`] (point sampling, no
    /// interpolation across those rows). [`load_topdown_atlases`] feeds this into
    /// [`load_with_settings`](AssetServer::load_with_settings); `pub(super)` so the sibling
    /// `topdown::test` module can pin the per-sheet decision without an app harness (the loaded
    /// image's sampler is unreachable in the headless `no_renderer` config — the image asset
    /// never finishes decoding without a render device).
    pub(super) fn sampler_override(self) -> Option<ImageSampler> {
        match self {
            Self::Portraits => Some(ImageSampler::nearest()),
            Self::Terrain | Self::Characters | Self::Effects => None,
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
    /// The grid layout over [`Self::image`], one entry per cell at the sheet's own
    /// tile size ([`SheetRole::tile_px`]).
    pub layout: Handle<TextureAtlasLayout>,
}

/// The presenter-owned, role-keyed atlas resource.
///
/// Holds, KEYED BY [`SheetRole`], the loaded [`SheetAtlas`] (image + layout handles)
/// for each render sheet. Built ONCE by [`load_topdown_atlases`] and present before
/// the S4/S5/S6 draw systems run, which read it to spawn sprites. A framework type
/// (`Resource`), exempt from no-bare-types; the role key it stores is the named
/// [`SheetRole`]. The GTW-278 [`Portraits`](SheetRole::Portraits) sheet rides the same
/// map — the status / hover panels read its image + layout to build a UI portrait node;
/// the deferred `ui` / `items` sheets join the same way.
#[derive(Resource, Debug, Clone)]
pub struct TopDownAtlases {
    /// One loaded [`SheetAtlas`] per loaded [`SheetRole`].
    sheets: HashMap<SheetRole, SheetAtlas>,
}

impl TopDownAtlases {
    /// The loaded [`SheetAtlas`] for `role`, if that role was loaded.
    ///
    /// Returns [`None`] for a role not in this loader's set (e.g. a deferred
    /// `ui` / `items` sheet) — callers match the [`Option`] rather than risk a panic.
    #[must_use]
    pub fn role(&self, role: SheetRole) -> Option<&SheetAtlas> {
        self.sheets.get(&role)
    }
}

/// Loads every sprite sheet and inserts the [`TopDownAtlases`] resource.
///
/// Runs ONCE (the [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) schedules
/// it in [`Startup`]): for each [`SheetRole`] it loads the PNG via
/// [`AssetServer::load`] (NOT the RON loader — that is `.ron`-only) and builds one
/// [`TextureAtlasLayout::from_grid`] per sheet at the role's OWN tile size
/// ([`SheetRole::tile_px`] — 16 for the render sheets, 32 for the GTW-278 portrait
/// sheet) and `(cols, rows)`, adding each layout to [`Assets<TextureAtlasLayout>`].
/// The resource is inserted via [`Commands`] so it is present before the draw slices
/// run.
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
        let layout =
            TextureAtlasLayout::from_grid(UVec2::splat(role.tile_px()), columns, rows, None, None);
        sheets.insert(
            role,
            SheetAtlas {
                image:  load_sheet_image(&asset_server, role),
                layout: layouts.add(layout),
            },
        );
    }

    commands.insert_resource(TopDownAtlases { sheets });
}

/// Loads `role`'s sheet image, choosing the texture sampler per role (GTW-295).
///
/// The per-role sampler DECISION is [`role.sampler_override()`](SheetRole::sampler_override):
/// the render sheets keep the asset server's DEFAULT sampler ([`None`]), and the
/// [`Portraits`](SheetRole::Portraits) sheet overrides it to NEAREST so its upscale into the
/// HUD portrait node point-samples instead of bilinearly blending the tiles' transparent-white
/// top rows into a whitish fringe (the GTW-295 white-line fix). `from_grid` stays the
/// geometrically-correct atlas carving; only the SAMPLER changes.
fn load_sheet_image(asset_server: &AssetServer, role: SheetRole) -> Handle<Image> {
    match role.sampler_override() {
        Some(sampler) => asset_server.load_with_settings(
            role.asset_path(),
            move |settings: &mut ImageLoaderSettings| {
                settings.sampler = sampler.clone();
            },
        ),
        None => asset_server.load(role.asset_path()),
    }
}
