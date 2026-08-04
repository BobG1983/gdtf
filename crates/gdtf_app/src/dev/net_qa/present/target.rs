use bevy::{
    image::Image,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

const CAPTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

#[derive(Resource, Debug, Clone, Deref)]
pub(in crate::dev::net_qa) struct QaCaptureTarget(Handle<Image>);

impl QaCaptureTarget {
    const fn new(handle: Handle<Image>) -> Self {
        Self(handle)
    }
}

pub(in crate::dev::net_qa) fn ensure_capture_target(
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
    let handle = images.add(image);
    commands.insert_resource(QaCaptureTarget::new(handle));
}
