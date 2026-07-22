//! The GTW-259 gamepad-software-cursor edge-pan input: the presenter-defined message
//! and its reader system.

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

/// The gamepad software cursor's SCREEN position, emitted by the input crate for the
/// presenter's edge-pan (GTW-259).
///
/// The presenter-owned input API for the gamepad edge-pan: a buffered [`Message`] (Bevy
/// 0.18 — buffered events are messages, `bevy-traps.md` #4) carrying the gamepad cursor's
/// SCREEN position (logical px, window origin TOP-LEFT, y-down — the same frame as the OS
/// cursor [`mouse_edge_dir`](super::dir::mouse_edge_dir) reads). The INPUT crate WRITES this
/// every update the gamepad
/// is the active pointer (after moving its software cursor);
/// [`pan_camera_on_gamepad_cursor_edge`] READS it and edge-pans the camera, REUSING the
/// SAME [`mouse_edge_dir`](super::dir::mouse_edge_dir) / [`pan_velocity`] helpers the
/// OS-cursor edge-pan uses.
///
/// Mirrors the [`HighlightRequest`](crate::HighlightRequest) message: the presenter, as the
/// CONSUMER, DEFINES this message (its input API), so the crate edge stays one-way
/// (`input → presenter`, never a cycle) — input can name a presenter-defined message, the
/// presenter never names input. The inner [`Vec2`] is framework-math plumbing (a raw screen
/// position — the GTW-249/250/251 carve-out for screen / direction coords), not a wrapped
/// domain scalar.
#[derive(Message, Deref, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursorMoved(Vec2);

impl GamepadCursorMoved {
    /// Build the gamepad-cursor-moved message from a raw screen position.
    #[must_use]
    pub const fn new(cursor: Vec2) -> Self {
        Self(cursor)
    }
}

/// `Update` (battle-gated): pan the [`WorldCamera`] when the GAMEPAD software cursor reaches
/// a screen edge (GTW-259), BEFORE
/// [`clamp_camera_to_bounds`](super::super::clamp_camera_to_bounds)
/// so it can never pan off the battlefield.
///
/// Drains the [`MessageReader<GamepadCursorMoved>`] (the input crate writes the gamepad
/// cursor's screen position every update the gamepad is the active pointer) and acts on the
/// LATEST position this update (the freshest cursor read), REUSING the GTW-271 viewport-edge
/// helper: [`viewport_edge_dir`]`(pos, `[`Camera::logical_viewport_rect`]`, edge_band)`
/// for the edge direction (with the screen-y → camera-y flip), [`pan_velocity`]`(dir,
/// pan_speed)` for the per-second velocity (the edge band + speed now read from the
/// hot-reloadable [`PanTuning`], see below), scaled by `time.delta_secs()`, added to the
/// camera `Transform.translation.xy` (z is kept — ground plane only). When NO message arrives
/// this update (the gamepad is not the active pointer, or the cursor is not near an edge) the
/// camera is left untouched (no pan). In `Mouse` mode the OS-cursor edge-pan in
/// [`pan_camera`](crate::pan_camera)
/// already covers the cursor, so this never double-pans.
///
/// GTW-271: like the mouse edge it now keys off the MAP VIEWPORT rect — a gamepad cursor in a
/// margin / over a panel is OUTSIDE the viewport rect and contributes nothing, so an edge-pan
/// never drags the battlefield beneath a panel. That viewport-inside gate REPLACED the GTW-262
/// `Interaction`-over-UI early-return.
///
/// GTW-299: the pan speed + edge band are now READ from the hot-reloadable [`PanTuning`]
/// resource ([`PanTuning::pan_speed`] / [`PanTuning::edge_band_px`]) rather than the old
/// `PAN_SPEED` / `EDGE_BAND_PX` consts — taken as `Option<Res<PanTuning>>` so a headless app
/// with no `AssetServer` falls back to the newtype [`Default`]s (the shipped 400 / 24) and
/// behaves exactly as before.
///
/// GTW-299 DWELL: like the mouse edge, the gamepad-cursor edge is now gated behind a
/// per-cursor-linger DWELL — the gamepad cursor must sit continuously in the edge band for at
/// least [`PanTuning::dwell_delay_seconds`] before it pans. Each frame the cursor is in-band the
/// gamepad accumulator in [`PanEdgeDwellState`] grows by `time.delta_secs()`; the frame it is NOT
/// in-band — no message this update (the gamepad is not the active pointer), or a message whose
/// cursor is in a margin / the viewport centre — the accumulator RESETS to zero, so a brief edge
/// graze never pans. A DISTINCT accumulator from the mouse edge keeps the two pointer sources from
/// interfering. The dwell state is taken as `Option<ResMut<PanEdgeDwellState>>` so a headless app
/// that never inserts it pans immediately, matching the pre-dwell behaviour.
///
/// Reads the [`MessageReader<GamepadCursorMoved>`], `Res<Time>` (`delta_secs()`), the optional
/// [`PanTuning`] / [`PanEdgeDwellState`], and the [`WorldCamera`]'s `&Camera` (its
/// [`Camera::logical_viewport_rect`] — the map sub-rect the edge bands against) alongside its
/// `&mut Transform`. Param-only (`Res` / `Query` / `MessageReader`), no `&mut World`
/// (`bevy-traps.md` #7); the battle gate (`bevy-traps.md` #1) and the
/// `.before(clamp_camera_to_bounds)` ordering (`bevy-traps.md` #3 — the clamp stays the last
/// writer) are applied at registration. It emits NO sim message — pan is presenter-only.
pub fn pan_camera_on_gamepad_cursor_edge(
    mut cursor_moves: MessageReader<GamepadCursorMoved>,
    time: Res<Time>,
    tuning: Option<Res<PanTuning>>,
    mut dwell: Option<ResMut<PanEdgeDwellState>>,
    mut cameras: Query<(&Camera, &mut Transform), With<WorldCamera>>,
) {
    // Act on only the LATEST gamepad cursor position this update — earlier reads are stale.
    // No message => the gamepad is not the active pointer => the cursor is not lingering at an
    // edge, so the gamepad dwell RESETS and no pan happens this frame.
    let Some(moved) = cursor_moves.read().last().copied() else {
        if let Some(state) = dwell.as_mut() {
            state.gamepad.reset();
        }
        return;
    };
    let cursor = *moved;

    // The hot-reloadable speed + edge band + dwell delay, or the shipped defaults when the tuning
    // table is absent (a headless app without an AssetServer never loads it) — behaviour unchanged.
    let pan_speed = tuning
        .as_ref()
        .map_or_else(PanSpeed::default, |t| t.pan_speed);
    let edge_band = tuning
        .as_ref()
        .map_or_else(EdgeBandPx::default, |t| t.edge_band_px);
    let dwell_delay = tuning
        .as_ref()
        .map_or_else(DwellDelaySeconds::default, |t| t.dwell_delay_seconds);

    // The gamepad-cursor edge direction this update (ZERO when no camera has a viewport rect, or
    // the cursor is in a margin / centre). Mirrors the single gamepad software cursor.
    let edge_dir = cameras
        .iter()
        .find_map(|(camera, _)| {
            let viewport = camera.logical_viewport_rect()?;
            let dir = viewport_edge_dir(cursor, viewport, edge_band);
            (dir != Vec2::ZERO).then_some(dir)
        })
        .unwrap_or(Vec2::ZERO);

    // Accumulate the in-band linger (or reset it when out of band) so the gamepad edge only pans
    // after a deliberate dwell — GTW-299 DWELL. With no dwell resource the edge is immediate.
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
        // The gamepad cursor edge-pans only when it is inside THIS camera's map viewport rect
        // and near an edge (a cursor in a margin / over UI contributes nothing — GTW-271 AC4).
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
