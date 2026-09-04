use bevy::prelude::*;
use cobalt_screenshot::{CapturePresentPlugin, WindowCapturePlugin};

pub(super) fn register_present(app: &mut App) {
    app.add_plugins(CapturePresentPlugin);
    app.add_plugins(WindowCapturePlugin);
}
