use bevy::prelude::*;
use gdtf_battle_input::InputSystems;

use crate::dev::net_qa::{router::route_requests, screenshot::drive_screenshots};

pub(super) fn register_consumers(app: &mut App) {
    app.add_systems(
        Update,
        drive_screenshots
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
}
