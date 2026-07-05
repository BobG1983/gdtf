//! The every-frame multi-source (keyboard / stick / mouse-edge + dwell) pan system.

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

/// `Update` (battle-gated): pan the [`WorldCamera`] each frame from the three navigation
/// sources, BEFORE the [`clamp_camera_to_bounds`](super::super::clamp_camera_to_bounds) clamp so
/// the camera can never pan off the battlefield.
///
/// GTW-250: sums the camera-space pan directions from the mouse at a screen edge
/// ([`mouse_edge_dir`](super::dir::mouse_edge_dir)), the keyboard ([`keyboard_pan_dir`] —
/// WASD + arrows), and the gamepad
/// RIGHT stick ([`stick_pan_dir`]), turns the summed direction into a per-second velocity via
/// [`pan_velocity`] (which normalises so diagonals are not faster than cardinals), scales by
/// `time.delta_secs()`, and adds the result to the camera `Transform.translation.xy` (z is
/// kept — top-down ground plane only, no zoom / multi-level). It emits NO sim message — the
/// camera is the VIEW, so pan navigation is presenter-only.
///
/// GTW-271: the MOUSE-EDGE source now keys off the MAP VIEWPORT rect, not the window. The
/// world map renders into a central sub-rect (the UI panels sit in the surrounding margins),
/// so the mouse edge-pans only when the cursor is INSIDE the viewport rect and within the edge
/// band ([`viewport_edge_dir`] reading [`Camera::logical_viewport_rect`]); a cursor in a margin
/// / over a panel contributes nothing. That viewport-inside gate REPLACED the GTW-262
/// `Interaction`-over-UI early-return (which caused the "green bar" and also wrongly suppressed
/// the keyboard / stick pan when the cursor merely rested on a panel). The keyboard + gamepad
/// stick are NOT cursor-bound, so they are unaffected by the cursor's position.
///
/// Reads the input sources via params: `Res<ButtonInput<KeyCode>>` (keyboard),
/// `Query<&Window, With<PrimaryWindow>>` (the cursor for the mouse edge — the cursor is `None`
/// when off the window, contributing nothing that frame), `Query<&Gamepad>` (the gamepad is an
/// ENTITY-component in Bevy 0.18, NOT the pre-0.15 `Res<Axis<GamepadAxis>>`), and the
/// [`WorldCamera`]'s `&Camera` (its [`Camera::logical_viewport_rect`] — the map sub-rect the
/// mouse edge bands against) alongside its `&mut Transform`. Movement scales by `Res<Time>`'s
/// `delta_secs()`.
///
/// GTW-299: the pan speed + edge band are now READ from the hot-reloadable [`PanTuning`]
/// resource ([`PanTuning::pan_speed`] / [`PanTuning::edge_band_px`]) rather than the old
/// `PAN_SPEED` / `EDGE_BAND_PX` consts, so editing `assets/core_tuning/pan.tuning.ron` retunes them
/// live. The resource is taken as `Option<Res<PanTuning>>` so a headless app with no
/// `AssetServer` (the resource never loads) falls back to the newtype [`Default`]s — the
/// shipped 400 / 24 — and behaves exactly as before (`bevy-traps.md` #1).
///
/// GTW-299 DWELL: the MOUSE-EDGE contribution is now gated behind a per-cursor-linger DWELL — the
/// cursor must rest continuously inside the edge band for at least
/// [`PanTuning::dwell_delay_seconds`] before it pans, so a brief graze / a click-release near the
/// border does NOT yank the camera. Each frame the cursor is in-band the mouse accumulator in
/// [`PanEdgeDwellState`] grows by `time.delta_secs()`; the frame it is NOT in-band (centre, in a
/// UI margin, or off the window) the accumulator RESETS to zero. The mouse edge is added to the
/// pan only once [`should_edge_pan_after_dwell`] is true. KEYBOARD and STICK are NOT dwell-gated —
/// they stay immediate (they are not cursor-bound). The dwell state is taken as
/// `Option<ResMut<PanEdgeDwellState>>` so a headless app that never inserts it still pans (it
/// falls back to a default-elapsed accumulator and behaves as before the dwell gate when the
/// resource is present with a zero threshold; the registration always inserts it).
///
/// Param-only (`Res` / `Query`), no `&mut World` (`bevy-traps.md` #7); the
/// battle gate (`bevy-traps.md` #1) and the `.before(clamp)` ordering (`bevy-traps.md` #3) are
/// applied at registration.
pub fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    tuning: Option<Res<PanTuning>>,
    mut dwell: Option<ResMut<PanEdgeDwellState>>,
    mut cameras: Query<(&Camera, &mut Transform), With<WorldCamera>>,
) {
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

    let mut dir = Vec2::ZERO;

    // Keyboard: WASD + arrow aliases. Not cursor-bound, NOT dwell-gated — always contributes.
    dir += keyboard_pan_dir(
        keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight),
    );

    // Gamepad RIGHT stick (the first connected pad), past the deadzone. Not cursor-bound, not
    // dwell-gated.
    if let Some(gamepad) = gamepads.iter().next() {
        dir += stick_pan_dir(gamepad.right_stick(), STICK_DEADZONE);
    }

    let cursor = windows
        .iter()
        .next()
        .and_then(bevy::window::Window::cursor_position);

    // The mouse-edge direction implied by the cursor this frame (ZERO when no cursor, in a margin,
    // or in the viewport centre). Computed once: the dwell accumulator tracks whether the cursor
    // is in-band (a non-ZERO edge dir) across the whole camera set, mirroring the single OS cursor.
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

    // Accumulate the in-band linger (or reset it the frame the cursor leaves the band) so the
    // mouse edge only pans after a deliberate dwell — GTW-299 DWELL. With no dwell resource
    // (a headless app that never inserts it) the mouse edge is treated as immediate, matching
    // the pre-dwell behaviour.
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
        // Mouse edge: only when the cursor is inside THIS camera's map viewport rect and near
        // an edge (GTW-271 AC4) AND it has dwelt there past the dwell delay (GTW-299 DWELL).
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
