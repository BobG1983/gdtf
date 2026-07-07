//! The role-keyed sheet/atlas model ([`TileIndex`] / [`SheetRole`] / [`SheetAtlas`] /
//! [`TopDownAtlases`]) and the one-shot atlas load ([`load_topdown_atlases`]).

use bevy::{
    image::{ImageLoaderSettings, ImageSampler, TextureAtlasLayout},
    platform::collections::HashMap,
    prelude::*,
};
use serde::{Deserialize, Serialize};

/// An index into a sprite sheet's atlas layout — WHICH grid tile a role draws.
///
/// A named newtype over `usize` (no-bare-types: an atlas index is a domain value, not
/// a bare `usize`), [`Deref`]ing to it so a consumer reads the index straight through.
/// `#[serde(transparent)]` so an authored role-table field parses as a bare integer
/// (`faction_0: 0`), not a one-field struct. [`Serialize`] (with the same transparency)
/// lets a round-trip-identity test re-serialize a loaded table. Rehomed here from the
/// retired terrain role table (GTW-665): the surviving consumers are the
/// [`CharacterRoles`](crate::CharacterRoles) / [`EffectRoles`](crate::EffectRoles)
/// tables over the character / effect sheets.
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
    /// Terrain tiles — `sprites/alt_tileset_terrain.png` (the S4 draw sheet).
    Terrain,
    /// Character tiles — `sprites/alt_tileset_characters.png` (the S5 draw sheet).
    Characters,
    /// Effect tiles — `sprites/alt_tileset_effects.png` (the S6 draw sheet).
    Effects,
    /// Portrait faces — `sprites/alt_tileset_portraits.png` (the GTW-278 HUD sheet): a
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
    /// both resolve to the shipped sheet. `pub` so integration tests can call the
    /// SAME path the runtime `load_topdown_atlases` uses — the GTW-447 load-state
    /// proof uses this to assert the new `sprites/` paths resolve to
    /// `LoadState::Loaded`.
    #[must_use]
    pub const fn asset_path(self) -> &'static str {
        match self {
            Self::Terrain => "sprites/alt_tileset_terrain.png",
            Self::Characters => "sprites/alt_tileset_characters.png",
            Self::Effects => "sprites/alt_tileset_effects.png",
            Self::Portraits => "sprites/alt_tileset_portraits.png",
        }
    }

    /// This sheet's grid shape as `(columns, rows)` of [`tile_px`](SheetRole::tile_px)
    /// cells.
    ///
    /// terrain 16×22 (352 tiles), characters 16×18 (288), effects 16×8 (128) — all of
    /// 16×16 px — and the GTW-278 portraits 10×10 (100 faces) of 32×32 px; the
    /// `from_grid` dimensions for [`load_topdown_atlases`]. `pub` (GTW-566 C7) so the
    /// content editor's terrain tile atlas reads the SAME sheet spec the presenter
    /// draws with instead of mirroring these dimensions as its own consts; the sibling
    /// `topdown::test` module pins the per-sheet grid through it too.
    #[must_use]
    pub const fn grid(self) -> (u32, u32) {
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
    /// straight to `from_grid`, not a domain quantity. `pub` (GTW-566 C7) so the
    /// content editor's terrain tile atlas reads the SAME per-cell size the presenter
    /// draws with instead of mirroring it as its own const; the sibling
    /// `topdown::test` module pins the per-sheet tile size through it too.
    #[must_use]
    pub const fn tile_px(self) -> u32 {
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
    /// One loaded [`SheetAtlas`] per loaded [`SheetRole`]. `pub(super)` (GTW-583) so
    /// the relocated `render::topdown::test` redrive tests keep building their
    /// `TopDownAtlases { sheets }` struct-literal fixtures — not a newtype inner, so
    /// the no-bare-types inner-privacy rule does not apply.
    pub(super) sheets: HashMap<SheetRole, SheetAtlas>,
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

    /// Which [`SheetRole`] (if any) the given image id belongs to.
    ///
    /// Scans the loaded sheets for the one whose [`SheetAtlas::image`] handle has this
    /// id, returning its role. The inverse of [`role`](Self::role): the image hot-reload
    /// (GTW-375 C4) reads an [`AssetEvent`](bevy::asset::AssetEvent)`<`[`Image`]`>` carrying
    /// only an [`AssetId<Image>`] and must map it back to the sheet that reloaded so it can
    /// log the sheet by name and (for [`Terrain`](SheetRole::Terrain)) force the terrain
    /// redraw. Returns [`None`] for an id that is not any loaded sheet's image (a portrait
    /// node, a one-off texture, a font atlas, …) so callers ignore unrelated reloads.
    #[must_use]
    pub fn sheet_role_for_image(&self, id: AssetId<Image>) -> Option<SheetRole> {
        self.sheets
            .iter()
            .find_map(|(role, sheet)| (sheet.image.id() == id).then_some(*role))
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
        // `load_with_settings` is deprecated in Bevy 0.19 in favor of the
        // `load_builder().with_settings(..).load(path)` chain.
        Some(sampler) => asset_server
            .load_builder()
            .with_settings(move |settings: &mut ImageLoaderSettings| {
                settings.sampler = sampler.clone();
            })
            .load(role.asset_path()),
        None => asset_server.load(role.asset_path()),
    }
}
