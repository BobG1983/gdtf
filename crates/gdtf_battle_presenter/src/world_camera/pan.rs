//! GTW-250 pan navigation (mouse-edge + keyboard + gamepad right stick) and the GTW-259
//! gamepad-cursor edge-pan: the view tunables, the pure direction/velocity helpers, the
//! presenter-defined cursor message, and the two pan systems.
//!
//! GTW-271 reworked the mouse / gamepad-cursor edge-pan to key off the MAP VIEWPORT rect
//! ([`Camera::logical_viewport_rect`]) rather than the whole window: the cursor only edge-pans
//! when it sits INSIDE the world camera's viewport sub-rect, so a cursor in a UI margin (over a
//! panel) never pans the map. That viewport-inside gate REPLACED the GTW-262
//! `Interaction`-over-UI suppression (which caused the "green bar" and over-suppressed
//! keyboard / stick panning), so this file no longer reads `bevy_ui::Interaction`.

use bevy::{input::gamepad::Gamepad, prelude::*, window::PrimaryWindow};

use super::marker::WorldCamera;

/// The pan speed of the [`WorldCamera`], in world units per second.
///
/// A VIEW tunable (how fast the camera glides under player navigation), not combat or
/// theme tuning — so it lives as a presenter-level const here with a doc-comment, NOT in
/// a `.ron` data file (the contract's view-config ruling). A newtype with a private inner
/// `f32` + derived [`Deref`](std::ops::Deref) (the house style for a domain value,
/// `no-bare-types.md`): the
/// speed is a domain quantity (world-units/sec), never a bare `f32`.
///
/// FOLLOW-UP (flagged per the contract): if the user later wants pan speed authored / hot-
/// swappable, promote this to a presenter view-config `.ron` resource — it is deliberately
/// a const for the first cut.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanSpeed(f32);

impl PanSpeed {
    /// Construct a [`PanSpeed`] from world-units-per-second.
    #[must_use]
    pub const fn new(units_per_second: f32) -> Self {
        Self(units_per_second)
    }
}

/// The mouse-edge band thickness, in logical screen pixels.
///
/// The cursor is "at an edge" (and pans the camera that way) when it sits within this many
/// logical pixels of a window edge. A VIEW tunable, so a presenter const with a doc-comment
/// (not `.ron`), and a newtype over a private `f32` ([`Deref`](std::ops::Deref)) per
/// `no-bare-types.md`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct EdgeBandPx(f32);

impl EdgeBandPx {
    /// Construct an [`EdgeBandPx`] from a logical-pixel band thickness.
    #[must_use]
    pub const fn new(pixels: f32) -> Self {
        Self(pixels)
    }
}

/// The gamepad-stick deadzone: a stick magnitude at or below this contributes no pan.
///
/// A unitless `[0, 1]` analog-stick magnitude threshold below which the right stick is
/// treated as centred (no drift). A VIEW tunable (a presenter const, not `.ron`) and a
/// newtype over a private `f32` ([`Deref`](std::ops::Deref)) per `no-bare-types.md`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct StickDeadzone(f32);

impl StickDeadzone {
    /// Construct a [`StickDeadzone`] from a unitless `[0, 1]` magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// The shipping pan speed: world units the camera glides per second under navigation.
///
/// A view const (see [`PanSpeed`]). Chosen so the whole 60-cell battlefield can be crossed
/// in a couple of seconds at the 16-px cell pitch ([`CELL_PX`](crate::CELL_PX)); FLAGGED for
/// data-driving later.
pub const PAN_SPEED: PanSpeed = PanSpeed::new(600.0);

/// The shipping mouse-edge band: cursor within this many logical pixels of a window edge
/// pans the camera that way (see [`EdgeBandPx`]).
pub const EDGE_BAND_PX: EdgeBandPx = EdgeBandPx::new(24.0);

/// The shipping gamepad right-stick deadzone (see [`StickDeadzone`]): a stick magnitude at
/// or below this is ignored so a resting stick never drifts the camera.
pub const STICK_DEADZONE: StickDeadzone = StickDeadzone::new(0.15);

/// The camera-space pan direction implied by the mouse cursor's position within the edge
/// bands of a `size`-pixel window, with the screen-y → camera-y flip baked in.
///
/// GTW-250 AC1. The window's origin is TOP-LEFT and screen-y grows DOWNWARD (Bevy's
/// `cursor_position`), while the camera's `+Y` is UP — so a cursor near the TOP edge
/// (`cursor.y < edge`) pans the camera UP (`+Y`) and a cursor near the BOTTOM edge
/// (`cursor.y > size.y - edge`) pans it DOWN (`-Y`); the x axis needs no flip (cursor left →
/// `-X`, right → `+X`). A cursor in NO band (the centre region) → [`Vec2::ZERO`]. The result
/// is a small integer-ish combination of unit axes (`-1`/`0`/`+1` per axis); the caller
/// normalises the summed multi-source direction so a corner is not faster than an edge.
///
/// Pure / total — no `App`, no `World` (unit-tested AC1). `Vec2` is framework-math plumbing
/// (the GTW-249 carve-out for raw screen / direction coords), not a domain newtype.
#[must_use]
pub fn mouse_edge_dir(cursor: Vec2, size: Vec2, edge: EdgeBandPx) -> Vec2 {
    let band = *edge;
    let mut dir = Vec2::ZERO;
    if cursor.x < band {
        dir.x -= 1.0;
    } else if cursor.x > size.x - band {
        dir.x += 1.0;
    }
    // Screen-y → camera-y FLIP: top band (small screen y) pans the camera UP (+Y); bottom
    // band (large screen y) pans it DOWN (-Y).
    if cursor.y < band {
        dir.y += 1.0;
    } else if cursor.y > size.y - band {
        dir.y -= 1.0;
    }
    dir
}

/// The camera-space pan direction implied by the cursor's position relative to the MAP
/// VIEWPORT rect — the GTW-271 viewport-aware edge gate.
///
/// GTW-271 AC4. The world map is rendered into a central sub-rectangle of the window (the
/// status-panel / action-bar / hover-panel margins surround it), so edge-pan must key off the
/// MAP viewport rect ([`Camera::logical_viewport_rect`], window-space LOGICAL px, directly
/// comparable to [`Window::cursor_position`]), NOT the whole window. A cursor OUTSIDE the
/// viewport rect (in a margin / over UI) → [`Vec2::ZERO`] (NO pan) — the viewport-inside gate
/// that REPLACED the GTW-262 `Interaction`-over-UI suppression. A cursor INSIDE the rect is
/// re-expressed VIEWPORT-RELATIVE (`cursor - viewport.min`) and fed through [`mouse_edge_dir`]
/// against the viewport SIZE (`viewport.size()`), so the same screen-y → camera-y flip and
/// edge-band logic apply, just measured from the viewport edges rather than the window edges.
///
/// Pure / total — no `App`, no `World` (unit-tested AC4). `Vec2` / [`Rect`] are framework-math
/// plumbing (the GTW-249 carve-out for raw screen / direction coords), not domain newtypes.
#[must_use]
pub fn viewport_edge_dir(cursor: Vec2, viewport: Rect, edge: EdgeBandPx) -> Vec2 {
    // OUTSIDE the map viewport (a UI margin / over a panel) → no pan. `Rect::contains` is
    // inclusive of the min edge and exclusive of the max — the cursor must be inside the map.
    if !viewport.contains(cursor) {
        return Vec2::ZERO;
    }
    // INSIDE: re-express the cursor relative to the viewport origin and band against the
    // viewport size, reusing the GTW-250 window-edge helper unchanged.
    mouse_edge_dir(cursor - viewport.min, viewport.size(), edge)
}

/// The camera-space pan direction implied by the WASD / arrow pan keys.
///
/// GTW-250 AC2. `up` (W / ↑) → `+Y`, `down` (S / ↓) → `-Y`, `left` (A / ←) → `-X`,
/// `right` (D / →) → `+X`; opposite keys CANCEL (W+S → no y, A+D → no x). The result is a
/// `-1`/`0`/`+1` combination per axis; the caller normalises the summed direction so a
/// diagonal (W+D) is not faster than a cardinal (W).
///
/// Pure / total — no `App`, no `World` (unit-tested AC2). `Vec2` is framework-math plumbing.
///
/// The four `bool` params are the contract's specified signature
/// (`keyboard_pan_dir(up, down, left, right: bool) -> Vec2`) — the four independent pan-key
/// pressed-states, each a genuine input the system fills from `ButtonInput<KeyCode>` (WASD +
/// arrow aliases). The localized `#[expect]` (the file's `cast_precision_loss` precedent)
/// keeps that exact, documented signature rather than narrowing it; the pedantic
/// `fn_params_excessive_bools` gate is the only thing it suppresses.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the four pan-key pressed states are the contract's specified keyboard_pan_dir \
              signature (up/down/left/right); they are independent inputs, not a flag soup"
)]
#[must_use]
pub fn keyboard_pan_dir(up: bool, down: bool, left: bool, right: bool) -> Vec2 {
    let x = f32::from(right) - f32::from(left);
    let y = f32::from(up) - f32::from(down);
    Vec2::new(x, y)
}

/// The camera-space pan direction implied by the gamepad RIGHT stick, after the deadzone.
///
/// GTW-250 AC3. A stick magnitude at or below `deadzone` → [`Vec2::ZERO`] (a resting stick
/// never drifts the camera); past the deadzone the stick passes through unchanged, with its
/// analog magnitude preserved (so a slight push pans slowly, a full push fast). Bevy's
/// `right_stick()` already reports stick-UP as `+Y`, matching the camera `+Y`-up convention,
/// so no flip is applied.
///
/// Pure / total — no `App`, no `World` (unit-tested AC3). `Vec2` is framework-math plumbing.
#[must_use]
pub fn stick_pan_dir(stick: Vec2, deadzone: StickDeadzone) -> Vec2 {
    if stick.length() <= *deadzone {
        return Vec2::ZERO;
    }
    stick
}

/// The per-second pan VELOCITY for a (summed, multi-source) `dir` at `speed`.
///
/// GTW-250 AC4. A [`Vec2::ZERO`] direction → zero velocity (no input → no drift). The
/// combined direction is NORMALISED when its length exceeds 1 so a diagonal keyboard combo
/// (length `√2`) is not faster than a cardinal one — the documented choice: keyboard / mouse
/// edges contribute unit axes and must not let diagonals out-run cardinals, while an analog
/// stick (magnitude < 1) keeps its sub-unit magnitude so a gentle push pans gently. The
/// result is `world-units/sec`; the caller multiplies by `delta_secs()` to get this frame's
/// translation delta.
///
/// Pure / total — no `App`, no `World` (unit-tested AC4). `Vec2` is framework-math plumbing.
#[must_use]
pub fn pan_velocity(dir: Vec2, speed: PanSpeed) -> Vec2 {
    let length = dir.length();
    if length == 0.0 {
        return Vec2::ZERO;
    }
    // Clamp the combined direction to at most unit length: a length > 1 (a diagonal of
    // unit-axis sources) is normalised so diagonals are not faster than cardinals; a length
    // <= 1 (a single axis, or a sub-unit analog stick) passes through so analog magnitude
    // still scales speed.
    let clamped = if length > 1.0 { dir / length } else { dir };
    clamped * *speed
}

/// The gamepad software cursor's SCREEN position, emitted by the input crate for the
/// presenter's edge-pan (GTW-259).
///
/// The presenter-owned input API for the gamepad edge-pan: a buffered [`Message`] (Bevy
/// 0.18 — buffered events are messages, `bevy-traps.md` #4) carrying the gamepad cursor's
/// SCREEN position (logical px, window origin TOP-LEFT, y-down — the same frame as the OS
/// cursor [`mouse_edge_dir`] reads). The INPUT crate WRITES this every update the gamepad
/// is the active pointer (after moving its software cursor);
/// [`pan_camera_on_gamepad_cursor_edge`] READS it and edge-pans the camera, REUSING the
/// SAME [`mouse_edge_dir`] / [`pan_velocity`] helpers the OS-cursor edge-pan uses.
///
/// Mirrors the [`HighlightRequest`](crate::HighlightRequest) seam: the presenter, as the
/// CONSUMER, DEFINES this message (its input API), so the crate edge stays one-way
/// (`input → presenter`, never a cycle) — input can name a presenter-defined message, the
/// presenter never names input. The inner [`Vec2`] is framework-math plumbing (a raw screen
/// position — the GTW-249/250/251 carve-out for screen / direction coords), not a wrapped
/// domain scalar.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursorMoved(pub Vec2);

/// `Update` (battle-gated): pan the [`WorldCamera`] each frame from the three navigation
/// sources, BEFORE the [`clamp_camera_to_bounds`](super::clamp_camera_to_bounds) clamp so
/// the camera can never pan off the battlefield.
///
/// GTW-250: sums the camera-space pan directions from the mouse at a screen edge
/// ([`mouse_edge_dir`]), the keyboard ([`keyboard_pan_dir`] — WASD + arrows), and the gamepad
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
/// `delta_secs()`. Param-only (`Res` / `Query`), no `&mut World` (`bevy-traps.md` #7); the
/// battle gate (`bevy-traps.md` #1) and the `.before(clamp)` ordering (`bevy-traps.md` #3) are
/// applied at registration.
pub fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut cameras: Query<(&Camera, &mut Transform), With<WorldCamera>>,
) {
    let mut dir = Vec2::ZERO;

    // Keyboard: WASD + arrow aliases. Not cursor-bound — always contributes.
    dir += keyboard_pan_dir(
        keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight),
    );

    // Gamepad RIGHT stick (the first connected pad), past the deadzone. Not cursor-bound.
    if let Some(gamepad) = gamepads.iter().next() {
        dir += stick_pan_dir(gamepad.right_stick(), STICK_DEADZONE);
    }

    let cursor = windows
        .iter()
        .next()
        .and_then(bevy::window::Window::cursor_position);
    for (camera, mut transform) in &mut cameras {
        let mut total = dir;
        // Mouse edge: only when the cursor is inside THIS camera's map viewport rect and near
        // an edge (a cursor in a margin / over UI contributes nothing — GTW-271 AC4).
        if let (Some(cursor), Some(viewport)) = (cursor, camera.logical_viewport_rect()) {
            total += viewport_edge_dir(cursor, viewport, EDGE_BAND_PX);
        }
        let velocity = pan_velocity(total, PAN_SPEED);
        if velocity == Vec2::ZERO {
            continue;
        }
        let delta = velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// `Update` (battle-gated): pan the [`WorldCamera`] when the GAMEPAD software cursor reaches
/// a screen edge (GTW-259), BEFORE [`clamp_camera_to_bounds`](super::clamp_camera_to_bounds)
/// so it can never pan off the battlefield.
///
/// Drains the [`MessageReader<GamepadCursorMoved>`] (the input crate writes the gamepad
/// cursor's screen position every update the gamepad is the active pointer) and acts on the
/// LATEST position this update (the freshest cursor read), REUSING the GTW-271 viewport-edge
/// helper: [`viewport_edge_dir`]`(pos, `[`Camera::logical_viewport_rect`]`, `[`EDGE_BAND_PX`]`)`
/// for the edge direction (with the screen-y → camera-y flip), [`pan_velocity`]`(dir,
/// `[`PAN_SPEED`]`)` for the per-second velocity, scaled by `time.delta_secs()`, added to the
/// camera `Transform.translation.xy` (z is kept — ground plane only). When NO message arrives
/// this update (the gamepad is not the active pointer, or the cursor is not near an edge) the
/// camera is left untouched (no pan). In `Mouse` mode the OS-cursor edge-pan in [`pan_camera`]
/// already covers the cursor, so this never double-pans.
///
/// GTW-271: like the mouse edge it now keys off the MAP VIEWPORT rect — a gamepad cursor in a
/// margin / over a panel is OUTSIDE the viewport rect and contributes nothing, so an edge-pan
/// never drags the battlefield beneath a panel. That viewport-inside gate REPLACED the GTW-262
/// `Interaction`-over-UI early-return.
///
/// Reads the [`MessageReader<GamepadCursorMoved>`], `Res<Time>` (`delta_secs()`), and the
/// [`WorldCamera`]'s `&Camera` (its [`Camera::logical_viewport_rect`] — the map sub-rect the
/// edge bands against) alongside its `&mut Transform`. Param-only (`Res` / `Query` /
/// `MessageReader`), no `&mut World` (`bevy-traps.md` #7); the battle gate (`bevy-traps.md` #1)
/// and the `.before(clamp_camera_to_bounds)` ordering (`bevy-traps.md` #3 — the clamp stays the
/// last writer) are applied at registration. It emits NO sim message — pan is presenter-only.
pub fn pan_camera_on_gamepad_cursor_edge(
    mut cursor_moves: MessageReader<GamepadCursorMoved>,
    time: Res<Time>,
    mut cameras: Query<(&Camera, &mut Transform), With<WorldCamera>>,
) {
    // Act on only the LATEST gamepad cursor position this update — earlier reads are stale.
    // No message => the gamepad is not the active pointer (or no edge) => no pan.
    let Some(GamepadCursorMoved(cursor)) = cursor_moves.read().last().copied() else {
        return;
    };

    for (camera, mut transform) in &mut cameras {
        // The gamepad cursor edge-pans only when it is inside THIS camera's map viewport rect
        // and near an edge (a cursor in a margin / over UI contributes nothing — GTW-271 AC4).
        let Some(viewport) = camera.logical_viewport_rect() else {
            continue;
        };
        let dir = viewport_edge_dir(cursor, viewport, EDGE_BAND_PX);
        let velocity = pan_velocity(dir, PAN_SPEED);
        if velocity == Vec2::ZERO {
            continue;
        }
        let delta = velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}
