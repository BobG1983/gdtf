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

#[must_use]
pub fn resolve_sprite<'a>(defs: &'a SpriteDefRegistry, name: &str) -> Option<&'a SpriteDef> {
    defs.def(&SpriteName::new(name.to_owned()))
}

#[must_use]
pub const fn source_parts(source: &SpriteSource) -> (&SpriteImagePath, Option<&SpriteRect>) {
    match source {
        SpriteSource::File(path) => (path, None),
        SpriteSource::Sheet { sheet, rect } => (sheet, Some(rect)),
    }
}

#[must_use]
pub fn source_urect(rect: &SpriteRect) -> URect {
    let min = UVec2::new(*rect.x, *rect.y);
    URect {
        min,
        max: min + UVec2::new(*rect.w, *rect.h),
    }
}

#[must_use]
pub fn source_px_size(source: &SpriteSource) -> Option<UVec2> {
    match source {
        SpriteSource::File(_) => None,
        SpriteSource::Sheet { rect, .. } => Some(UVec2::new(*rect.w, *rect.h)),
    }
}

#[must_use]
pub fn single_rect_layout(region: URect) -> TextureAtlasLayout {
    let mut layout = TextureAtlasLayout::new_empty(region.max);
    layout.add_texture(region);
    layout
}

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

#[derive(Resource, Debug, Clone)]
pub struct MissingTileTexture(Handle<Image>);

impl MissingTileTexture {
        #[must_use]
    pub fn handle(&self) -> Handle<Image> {
        self.0.clone()
    }
}

const MISSING_TEXEL: [u8; 4] = [200, 60, 200, 255];

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
