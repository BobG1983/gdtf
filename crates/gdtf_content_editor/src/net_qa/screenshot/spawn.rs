use bevy::{
    camera::RenderTarget,
    image::Image,
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};
use gdtf_screenshot::CapturePath;

use super::config::EditorShotSource;

pub(super) fn spawn_capture(
    path: &CapturePath,
    source: &EditorShotSource,
    commands: &mut Commands,
) {
    purge_existing(path);
    let screenshot = match source {
        EditorShotSource::PrimaryWindow => Screenshot::primary_window(),
        EditorShotSource::Offscreen(target) if target.handle == Handle::<Image>::default() => {
            Screenshot::primary_window()
        }
        EditorShotSource::Offscreen(target) => Screenshot(RenderTarget::Image(target.clone())),
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

fn purge_existing(path: &CapturePath) {
    drop(std::fs::remove_file(&**path));
}
