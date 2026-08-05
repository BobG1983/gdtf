use bevy::prelude::*;
use gdtf_screenshot::{CapturePresentPlugin, PresentSystems};

use crate::dev::net_qa::present::{mark_ui_default_camera, retarget_cameras_to_offscreen};

pub(super) fn register_present(app: &mut App) {
    app.add_plugins(CapturePresentPlugin);
    app.add_systems(
        Update,
        (retarget_cameras_to_offscreen, mark_ui_default_camera)
            .chain()
            .in_set(PresentSystems),
    );
}
