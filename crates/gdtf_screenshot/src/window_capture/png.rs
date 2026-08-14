//! Turn a finished readback into the PNG at the capture's own path.

use bevy::{
    image::{Image, TextureFormatPixelInfo},
    prelude::*,
    render::{
        gpu_readback::ReadbackComplete,
        render_resource::{Extent3d, TextureFormat},
        renderer::RenderDevice,
    },
};

use super::shot::CaptureShot;
use crate::path::CapturePath;

// Bytes per pixel of the format the plugin renders into.
const SOURCE_PIXEL_BYTES: usize = 4;

// Bytes per pixel written to the PNG; the alpha channel is dropped.
const PNG_PIXEL_BYTES: usize = 3;

// Despawning removes the Readback, which would otherwise re-read every frame.
pub(super) fn write_capture_png(
    complete: On<ReadbackComplete>,
    shots: Query<&CaptureShot>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    let entity = complete.entity;
    let Ok(shot) = shots.get(entity) else {
        return;
    };
    commands.entity(entity).try_despawn();
    let Some(image) = images.get(&**shot.image()) else {
        warn!("gdtf_screenshot: the capture image is gone, so no PNG was written");
        return;
    };
    let size = image.texture_descriptor.size;
    let Some(pixels) = frame_rgb(size, image.texture_descriptor.format, &complete.data) else {
        warn!(
            bytes = complete.data.len(),
            "gdtf_screenshot: the readback bytes do not fit the capture image, so no PNG was \
             written",
        );
        return;
    };
    if !write_png(shot.path(), size, pixels) {
        warn!(path = %shot.path().display(), "gdtf_screenshot: writing the capture PNG failed");
    }
}

// Strip the readback buffer's row padding and the alpha channel.
pub(super) fn frame_rgb(size: Extent3d, format: TextureFormat, data: &[u8]) -> Option<Vec<u8>> {
    if format.pixel_size().ok() != Some(SOURCE_PIXEL_BYTES) {
        return None;
    }
    let width = size.width as usize;
    let height = size.height as usize;
    let row = width * SOURCE_PIXEL_BYTES;
    let padded = RenderDevice::align_copy_bytes_per_row(row);
    if data.len() < padded * height {
        return None;
    }
    let mut pixels = Vec::with_capacity(width * height * PNG_PIXEL_BYTES);
    for y in 0..height {
        let start = y * padded;
        for x in 0..width {
            let texel = start + x * SOURCE_PIXEL_BYTES;
            pixels.extend_from_slice(&data[texel..texel + PNG_PIXEL_BYTES]);
        }
    }
    Some(pixels)
}

fn write_png(path: &CapturePath, size: Extent3d, pixels: Vec<u8>) -> bool {
    let Some(frame) = image::RgbImage::from_raw(size.width, size.height, pixels) else {
        return false;
    };
    frame
        .save_with_format(&**path, image::ImageFormat::Png)
        .is_ok()
}
