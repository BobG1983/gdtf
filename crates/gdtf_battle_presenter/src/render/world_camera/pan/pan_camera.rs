//! Keyboard, stick, and mouse-edge camera pan system.

use bevy::{input::gamepad::Gamepad, prelude::*, window::PrimaryWindow};

use super::{
    super::{
        dwell::{PanEdgeDwellState, should_edge_pan_after_dwell},
        marker::WorldCamera,
        tuning::{DwellDelaySeconds, PanTuning},
    },
    dir::{keyboard_pan_dir, pan_velocity, stick_pan_dir, viewport_edge_dir},
    tunables::{EdgeBandPx, PanSpeed, STICK_DEADZONE},
};

/// Apply keyboard, gamepad stick, and dwelt mouse-edge pan to the world camera.
pub fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    tuning: Option<Res<PanTuning>>,
    mut dwell: Option<ResMut<PanEdgeDwellState>>,
    mut cameras: Query<(&Camera, &mut Transform), With<WorldCamera>>,
) {
    let pan_speed = tuning
        .as_ref()
        .map_or_else(PanSpeed::default, |t| t.pan_speed);
    let edge_band = tuning
        .as_ref()
        .map_or_else(EdgeBandPx::default, |t| t.edge_band_px);
    let dwell_delay = tuning
        .as_ref()
        .map_or_else(DwellDelaySeconds::default, |t| t.dwell_delay_seconds);

    let mut dir = Vec2::ZERO;

    dir += keyboard_pan_dir(
        keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight),
    );

    if let Some(gamepad) = gamepads.iter().next() {
        dir += stick_pan_dir(gamepad.right_stick(), STICK_DEADZONE);
    }

    let cursor = windows
        .iter()
        .next()
        .and_then(bevy::window::Window::cursor_position);

    let edge_dir = cameras
        .iter()
        .find_map(|(camera, _)| {
            let (Some(cursor), Some(viewport)) = (cursor, camera.logical_viewport_rect()) else {
                return None;
            };
            let dir = viewport_edge_dir(cursor, viewport, edge_band);
            (dir != Vec2::ZERO).then_some(dir)
        })
        .unwrap_or(Vec2::ZERO);

    let mouse_dwelt = match dwell.as_mut() {
        Some(state) => {
            if edge_dir == Vec2::ZERO {
                state.mouse.reset();
            } else {
                state.mouse.accumulate(time.delta_secs());
            }
            should_edge_pan_after_dwell(state.mouse, dwell_delay)
        }
        None => true,
    };

    for (camera, mut transform) in &mut cameras {
        let mut total = dir;
        if mouse_dwelt
            && let (Some(cursor), Some(viewport)) = (cursor, camera.logical_viewport_rect())
        {
            total += viewport_edge_dir(cursor, viewport, edge_band);
        }
        let velocity = pan_velocity(total, pan_speed);
        if velocity == Vec2::ZERO {
            continue;
        }
        let delta = velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}
