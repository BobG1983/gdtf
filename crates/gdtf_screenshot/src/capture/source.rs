//! Where a capture reads its pixels from.

use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
};

/// Render target a capture reads.
#[derive(Resource, Clone, Debug)]
pub enum CaptureSource {
    /// Read the primary window's swapchain.
    PrimaryWindow,
    /// Read an offscreen image.
    Offscreen(ImageRenderTarget),
}

impl Default for CaptureSource {
    fn default() -> Self {
        Self::Offscreen(ImageRenderTarget::from(Handle::<Image>::default()))
    }
}

/// Whether `current` renders into exactly `target` — handle and scale factor both.
#[must_use]
pub fn aims_at(current: &RenderTarget, target: &ImageRenderTarget) -> bool {
    matches!(current, RenderTarget::Image(image) if image == target)
}
