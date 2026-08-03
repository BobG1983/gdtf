use bevy::prelude::*;
use gdtf_battle_sim::{battle::PlayerFaction, prelude::BattleInProgress};

use crate::{
    GamepadCursorMoved, PanEdgeDwellState, clamp_camera_to_bounds, frame_camera_on_units,
    pan_camera, pan_camera_on_gamepad_cursor_edge,
};

pub(super) fn register_camera_framing_systems(app: &mut App) {
    let battle_gate =
        resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>);
    app.init_resource::<PanEdgeDwellState>();
    app.add_message::<GamepadCursorMoved>().add_systems(
        Update,
        (
            frame_camera_on_units,
            pan_camera,
            pan_camera_on_gamepad_cursor_edge,
            clamp_camera_to_bounds
                .after(frame_camera_on_units)
                .after(pan_camera)
                .after(pan_camera_on_gamepad_cursor_edge),
        )
            .run_if(battle_gate),
    );
}
