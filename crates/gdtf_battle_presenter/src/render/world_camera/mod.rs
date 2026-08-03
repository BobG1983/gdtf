mod dwell;
mod framing;
mod marker;
mod pan;
mod tuning;

#[cfg(test)]
mod test;

pub use dwell::{DwellElapsed, PanEdgeDwellState, should_edge_pan_after_dwell};
pub use framing::{camera_focus, clamp_camera, clamp_camera_to_bounds, frame_camera_on_units};
pub use marker::{WORLD_RENDER_LAYER, WorldCamera, despawn_world_camera, spawn_world_camera};
pub use pan::{
    EdgeBandPx, GamepadCursorMoved, PanSpeed, STICK_DEADZONE, StickDeadzone, keyboard_pan_dir,
    mouse_edge_dir, pan_camera, pan_camera_on_gamepad_cursor_edge, pan_velocity, stick_pan_dir,
    viewport_edge_dir,
};
pub(crate) use tuning::register_pan_tuning_hot_ron;
pub use tuning::{BoundsMarginWorld, DwellDelaySeconds, PanTuning};
