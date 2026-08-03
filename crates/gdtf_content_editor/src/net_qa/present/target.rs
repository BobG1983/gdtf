use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

use crate::net_qa::screenshot::EditorShotSource;

const CAPTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

#[derive(Resource, Debug, Clone, Deref)]
pub(in crate::net_qa) struct EditorQaCaptureTarget(ImageRenderTarget);

impl EditorQaCaptureTarget {
        pub(in crate::net_qa) const fn new(target: ImageRenderTarget) -> Self {
        Self(target)
    }
}

pub(in crate::net_qa) fn aims_at(current: &RenderTarget, target: &ImageRenderTarget) -> bool {
    matches!(current, RenderTarget::Image(image) if image == target)
}

pub(in crate::net_qa) fn ensure_editor_capture_target(
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
    commands.insert_resource(EditorShotSource::Offscreen(target.clone()));
    commands.insert_resource(EditorQaCaptureTarget::new(target));
}
