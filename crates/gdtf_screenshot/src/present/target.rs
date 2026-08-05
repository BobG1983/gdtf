//! The offscreen image the app renders into so a capture reads a real texture.

use bevy::{
    camera::ImageRenderTarget,
    image::Image,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

use crate::capture::CaptureSource;

// The format bevy_egui's extract picks for a non-HDR view; a mismatch renders nothing.
const CAPTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// Offscreen render target the app draws into and captures read.
#[derive(Resource, Debug, Clone, Deref)]
pub struct QaCaptureTarget(ImageRenderTarget);

impl QaCaptureTarget {
    /// Wrap the offscreen target.
    #[must_use]
    pub const fn new(target: ImageRenderTarget) -> Self {
        Self(target)
    }
}

pub(super) fn ensure_capture_target(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = window.physical_size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    let mut image = Image::new_target_texture(size.x, size.y, CAPTURE_FORMAT, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = ImageRenderTarget {
        handle:       images.add(image),
        scale_factor: window.scale_factor(),
    };
    commands.insert_resource(CaptureSource::Offscreen(target.clone()));
    commands.insert_resource(QaCaptureTarget::new(target));
}
