//! Prepare the path on disk and spawn the readback the capture reads back.

use bevy::{prelude::*, render::gpu_readback::Readback};

use crate::{
    path::CapturePath,
    window_capture::{CaptureImage, CaptureShot},
};

pub(super) fn spawn_capture(
    path: &CapturePath,
    image: &CaptureImage,
    commands: &mut Commands<'_, '_>,
) {
    purge_existing(path);
    commands.spawn((
        Readback::texture((**image).clone()),
        CaptureShot::new(path.clone(), image.clone()),
    ));
}

pub(super) fn ensure_dir(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
}

pub(super) fn purge_existing(path: &CapturePath) {
    drop(std::fs::remove_file(&**path));
}
