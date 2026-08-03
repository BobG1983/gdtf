//! Gamepad virtual-cursor edge pan.

use bevy::prelude::*;

use super::{
    super::{
        dwell::{PanEdgeDwellState, should_edge_pan_after_dwell},
        marker::WorldCamera,
        tuning::{DwellDelaySeconds, PanTuning},
    },
    dir::{pan_velocity, viewport_edge_dir},
    tunables::{EdgeBandPx, PanSpeed},
};

/// Message carrying the gamepad virtual cursor position in window space.
#[derive(Message, Deref, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursorMoved(Vec2);

impl GamepadCursorMoved {
    /// Build from a cursor position.
    #[must_use]
    pub const fn new(cursor: Vec2) -> Self {
        Self(cursor)
    }
}

/// Pan the world camera when the gamepad cursor dwells on a viewport edge.
pub fn pan_camera_on_gamepad_cursor_edge(
    mut cursor_moves: MessageReader<GamepadCursorMoved>,
    time: Res<Time>,
    tuning: Option<Res<PanTuning>>,
    mut dwell: Option<ResMut<PanEdgeDwellState>>,
    mut cameras: Query<(&Camera, &mut Transform), With<WorldCamera>>,
) {
    let Some(moved) = cursor_moves.read().last().copied() else {
        if let Some(state) = dwell.as_mut() {
            state.gamepad.reset();
        }
        return;
    };
    let cursor = *moved;

    let pan_speed = tuning
        .as_ref()
        .map_or_else(PanSpeed::default, |t| t.pan_speed);
    let edge_band = tuning
        .as_ref()
        .map_or_else(EdgeBandPx::default, |t| t.edge_band_px);
    let dwell_delay = tuning
        .as_ref()
        .map_or_else(DwellDelaySeconds::default, |t| t.dwell_delay_seconds);

    let edge_dir = cameras
        .iter()
        .find_map(|(camera, _)| {
            let viewport = camera.logical_viewport_rect()?;
            let dir = viewport_edge_dir(cursor, viewport, edge_band);
            (dir != Vec2::ZERO).then_some(dir)
        })
        .unwrap_or(Vec2::ZERO);

    let gamepad_dwelt = match dwell.as_mut() {
        Some(state) => {
            if edge_dir == Vec2::ZERO {
                state.gamepad.reset();
            } else {
                state.gamepad.accumulate(time.delta_secs());
            }
            should_edge_pan_after_dwell(state.gamepad, dwell_delay)
        }
        None => true,
    };
    if !gamepad_dwelt {
        return;
    }

    for (camera, mut transform) in &mut cameras {
        let Some(viewport) = camera.logical_viewport_rect() else {
            continue;
        };
        let dir = viewport_edge_dir(cursor, viewport, edge_band);
        let velocity = pan_velocity(dir, pan_speed);
        if velocity == Vec2::ZERO {
            continue;
        }
        let delta = velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}
