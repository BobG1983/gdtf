//! The image the window's frame is rendered into, kept at the window's size.

use bevy::{
    camera::NormalizedRenderTarget,
    image::Image,
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{TextureFormat, TextureUsages},
    },
    window::{PrimaryWindow, WindowRef},
};

/// Format the capture image carries and the PNG writer reads back.
pub(super) const CAPTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// Image the window's frame is rendered into while a capture is in flight.
#[derive(Resource, Debug, Clone, Default, Deref, ExtractResource)]
pub struct CaptureImage(Handle<Image>);

impl CaptureImage {
    /// Wrap the image a capture reads.
    #[must_use]
    pub const fn new(handle: Handle<Image>) -> Self {
        Self(handle)
    }
}

/// The primary window's render target — the attachment a capture overrides.
#[derive(Resource, Debug, Clone, Deref, ExtractResource)]
pub(super) struct CaptureWindowTarget(NormalizedRenderTarget);

impl CaptureWindowTarget {
    /// Wrap the window target a capture overrides.
    #[must_use]
    const fn new(target: NormalizedRenderTarget) -> Self {
        Self(target)
    }
}

// Rebuilds the image whenever the primary window's physical size changes.
pub(super) fn sync_capture_target(
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    current: Res<CaptureImage>,
    aimed_at: Option<Res<CaptureWindowTarget>>,
    mut commands: Commands,
) {
    let Ok((entity, window)) = windows.single() else {
        return;
    };
    if let Some(window_ref) = WindowRef::Primary.normalize(Some(entity)) {
        let target = NormalizedRenderTarget::Window(window_ref);
        if aimed_at.is_none_or(|held| **held != target) {
            commands.insert_resource(CaptureWindowTarget::new(target));
        }
    }

    let size = window.physical_size().max(UVec2::ONE);
    if images
        .get(&**current)
        .is_some_and(|held| held.size() == size)
    {
        return;
    }
    let mut image = Image::new_target_texture(size.x, size.y, CAPTURE_FORMAT, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    commands.insert_resource(CaptureImage::new(images.add(image)));
}
