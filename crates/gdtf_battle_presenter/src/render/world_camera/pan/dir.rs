//! The pure direction / velocity helpers the pan systems compose.

use bevy::prelude::*;

use super::tunables::{EdgeBandPx, PanSpeed, StickDeadzone};

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
