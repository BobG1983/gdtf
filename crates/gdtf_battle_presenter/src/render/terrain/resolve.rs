//! The **def-driven sprite resolution** (GTW-665) — the ONE place a terrain
//! `graphic_name` resolves to drawable pixels, shared by BOTH consumers (the
//! battle draw here and the content editor's preview / thumbnails).
//!
//! The retired role table resolved `graphic_name → TileRole → hardcoded atlas
//! index`; this module resolves `graphic_name → `[`SpriteDef`]` → texture +
//! rect + anchor` through the GTW-663 [`SpriteDefRegistry`]
//! (`assets/content/sprites/*.spritedef.ron`). The
//! [`TileRole`](super::roles::TileRole) enum STAYS the closed renderer
//! vocabulary (the sim-fact → role fallback mapping and the editor's picker
//! filter) — what retired is the role→index TABLE, not the role concern seam.
//!
//! A name that resolves NO def draws the LOUD [`MissingTileTexture`] magenta
//! marker (C4 — the Level-Rail missing-marker precedent): never a panic, never
//! an invisible tile.

use bevy::{
    asset::RenderAssetUsages,
    image::{Image, TextureAtlasLayout},
    math::{URect, UVec2, Vec2},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use gdtf_content_families::sprites::{
    SpriteDef, SpriteDefRegistry, SpriteImagePath, SpriteName, SpriteRect, SpriteSource,
};

/// Resolve a graphic name to its [`SpriteDef`] — THE resolution (GTW-665 C1):
/// the sim's authored `graphic_name` (or a presenter-owned
/// [`TileRole`](super::roles::TileRole) key via
/// [`as_key`](super::roles::TileRole::as_key)) is a foreign key by NAME into
/// the [`SpriteDefRegistry`]. [`None`] means the def is MISSING (possible
/// mid-authoring — the integrity edge warns but load still exits); the caller
/// draws the loud [`MissingTileTexture`] fallback, never nothing.
#[must_use]
pub fn resolve_sprite<'a>(defs: &'a SpriteDefRegistry, name: &str) -> Option<&'a SpriteDef> {
    defs.def(&SpriteName::new(name.to_owned()))
}

/// Split a [`SpriteSource`] into its image path + OPTIONAL pixel region: a
/// [`Sheet`](SpriteSource::Sheet) source is its sheet path + rect; a
/// [`File`](SpriteSource::File) source is the whole image (no rect).
#[must_use]
pub const fn source_parts(source: &SpriteSource) -> (&SpriteImagePath, Option<&SpriteRect>) {
    match source {
        SpriteSource::File(path) => (path, None),
        SpriteSource::Sheet { sheet, rect } => (sheet, Some(rect)),
    }
}

/// A [`SpriteRect`] as the pixel-space [`URect`] the render pipeline consumes
/// (min = the authored top-left, max = min + extent).
#[must_use]
pub fn source_urect(rect: &SpriteRect) -> URect {
    let min = UVec2::new(*rect.x, *rect.y);
    URect {
        min,
        max: min + UVec2::new(*rect.w, *rect.h),
    }
}

/// The source's own pixel extent, where it is cheaply knowable: a
/// [`Sheet`](SpriteSource::Sheet) source is its authored rect's `w × h`; a
/// [`File`](SpriteSource::File) source is [`None`] (the image decodes async —
/// the caller may substitute the LOADED dims from [`Assets<Image>`] when
/// available, the GTW-664 layered-knowledge precedent).
#[must_use]
pub fn source_px_size(source: &SpriteSource) -> Option<UVec2> {
    match source {
        SpriteSource::File(_) => None,
        SpriteSource::Sheet { rect, .. } => Some(UVec2::new(*rect.w, *rect.h)),
    }
}

/// A single-rect [`TextureAtlasLayout`] over the def's authored pixel `region` — the
/// material-path carrier of the rect (GTW-665 C1): the terrain material bakes
/// its UV transform from `layout.textures[index]`, so a one-entry layout at
/// index `0` expresses an ARBITRARY authored rect with the exact same math the
/// retired grid layout produced for a grid-aligned one (identical pixels for
/// the seeded defs). The layout's declared `size` is the region's far corner —
/// the UV bake never reads it (it divides by the LOADED image's dims), it only
/// needs to contain the region.
#[must_use]
pub fn single_rect_layout(region: URect) -> TextureAtlasLayout {
    let mut layout = TextureAtlasLayout::new_empty(region.max);
    layout.add_texture(region);
    layout
}

/// The world-space offset of a sprite's CENTER from its cell position, so the
/// authored [`anchor`](SpriteDef::anchor) (ground-contact/pivot, sprite-local
/// pixels from the top-left) sits exactly ON the cell position (GTW-665 C2).
///
/// `sprite_px` is the sprite's own pixel extent ([`source_px_size`], or the
/// loaded image dims for a `File` source); `drawn_size` is the on-screen world
/// size the sprite is drawn at (the `CELL_PX` recipe). A CENTER anchor
/// (`w/2, h/2` — every GTW-663 seed) yields `Vec2::ZERO`, preserving the
/// pre-swap centered placement pixel-for-pixel. Local pixel `y` grows DOWN;
/// world `y` grows UP, hence the flipped `y` term. A degenerate zero extent
/// yields `Vec2::ZERO` (no divide-by-zero).
#[must_use]
pub fn anchor_world_offset(def: &SpriteDef, sprite_px: UVec2, drawn_size: Vec2) -> Vec2 {
    if sprite_px.x == 0 || sprite_px.y == 0 {
        return Vec2::ZERO;
    }
    let px = sprite_px.as_vec2();
    let anchor = UVec2::new(*def.anchor.x, *def.anchor.y).as_vec2();
    let center = px * 0.5;
    let scale = drawn_size / px;
    Vec2::new(
        (center.x - anchor.x) * scale.x,
        (anchor.y - center.y) * scale.y,
    )
}

/// The loud "missing sprite" marker texture (GTW-665 C4) — a 1×1 MAGENTA image
/// every unresolvable `graphic_name` draws (the Level-Rail `FALLBACK_HUE`
/// precedent): unmistakably "unresolved", never a panic, never invisible.
///
/// Built ONCE at `Startup` by [`setup_missing_tile_texture`] and read by the
/// terrain draw / swaps / link draw whenever [`resolve_sprite`] yields no def.
#[derive(Resource, Debug, Clone)]
pub struct MissingTileTexture(Handle<Image>);

impl MissingTileTexture {
    /// The marker image handle (cloned for a sprite / material).
    #[must_use]
    pub fn handle(&self) -> Handle<Image> {
        self.0.clone()
    }
}

/// The marker's one texel — opaque magenta (the Level-Rail missing hue).
const MISSING_TEXEL: [u8; 4] = [200, 60, 200, 255];

/// `Startup` (gated on [`Assets<Image>`] existing, so a `MinimalPlugins` app
/// no-ops — `bevy-traps.md` #1): mint the 1×1 magenta [`MissingTileTexture`]
/// and insert the resource. Param-only (`bevy-traps.md` #7).
pub fn setup_missing_tile_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_fill(
        Extent3d {
            width:                 1,
            height:                1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &MISSING_TEXEL,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    commands.insert_resource(MissingTileTexture(images.add(image)));
}
