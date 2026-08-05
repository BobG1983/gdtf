//! Prepare the path on disk and spawn Bevy's screenshot entity.

use bevy::{
    camera::RenderTarget,
    image::Image,
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};

use super::source::CaptureSource;
use crate::path::CapturePath;

pub(super) fn spawn_capture(
    path: &CapturePath,
    source: &CaptureSource,
    commands: &mut Commands<'_, '_>,
) {
    purge_existing(path);
    let screenshot = match source {
        CaptureSource::PrimaryWindow => Screenshot::primary_window(),
        CaptureSource::Offscreen(target) if target.handle == Handle::<Image>::default() => {
            Screenshot::primary_window()
        }
        CaptureSource::Offscreen(target) => Screenshot(RenderTarget::Image(target.clone())),
    };
    commands
        .spawn(screenshot)
        .observe(save_to_disk((**path).clone()));
}

pub(super) fn ensure_dir(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
}

pub(super) fn purge_existing(path: &CapturePath) {
    drop(std::fs::remove_file(&**path));
}
