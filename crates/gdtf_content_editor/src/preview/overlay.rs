//! authored asset; the patterns are structural, not art).

use bevy::{
    asset::RenderAssetUsages,
    image::{Image, ImageSampler},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const STIPPLE_PX: u32 = 8;

const STIPPLE_BLOCK_PX: u32 = 2;

const VOID_GRID_PX: u32 = 16;

const WHITE_TEXEL: [u8; 4] = [255, 255, 255, 255];

const CLEAR_TEXEL: [u8; 4] = [0, 0, 0, 0];

#[derive(Resource, Debug, Clone)]
pub(crate) struct PreviewOverlayImages {
    stipple:   Handle<Image>,
    void_grid: Handle<Image>,
}

impl PreviewOverlayImages {
    pub(crate) fn stipple(&self) -> Handle<Image> {
        self.stipple.clone()
    }

    pub(crate) fn void_grid(&self) -> Handle<Image> {
        self.void_grid.clone()
    }
}

pub(crate) fn create_preview_overlay_images(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(PreviewOverlayImages {
        stipple:   images.add(stipple_image()),
        void_grid: images.add(void_grid_image()),
    });
}

pub(crate) fn remove_preview_overlay_images(mut commands: Commands) {
    commands.remove_resource::<PreviewOverlayImages>();
}

fn pattern_image(edge: u32, on: impl Fn(u32, u32) -> bool) -> Image {
    let mut data = Vec::with_capacity((edge * edge * 4) as usize);
    for y in 0..edge {
        for x in 0..edge {
            let texel = if on(x, y) { WHITE_TEXEL } else { CLEAR_TEXEL };
            data.extend_from_slice(&texel);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width:                 edge,
            height:                edge,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn stipple_image() -> Image {
    pattern_image(STIPPLE_PX, |x, y| {
        ((x / STIPPLE_BLOCK_PX) + (y / STIPPLE_BLOCK_PX)).is_multiple_of(2)
    })
}

fn void_grid_image() -> Image {
    pattern_image(VOID_GRID_PX, |x, y| x == 0 || y == 0)
}
