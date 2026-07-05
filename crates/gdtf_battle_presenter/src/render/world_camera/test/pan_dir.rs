//! Tests of the pure pan-direction / velocity helpers (mirrors `pan.rs`'s pure fns).

use bevy::prelude::*;

use super::super::pan::{
    EdgeBandPx, PanSpeed, StickDeadzone, keyboard_pan_dir, mouse_edge_dir, pan_velocity,
    stick_pan_dir, viewport_edge_dir,
};

// -----------------------------------------------------------------------------
// GTW-250 — pan-navigation pure helpers (AC1–AC4).
// -----------------------------------------------------------------------------

/// A test window size for the mouse-edge helper.
const SIZE: Vec2 = Vec2::new(800.0, 600.0);
/// A test mouse-edge band.
const EDGE: EdgeBandPx = EdgeBandPx::new(20.0);

/// AC1 — `mouse_edge_dir` maps each edge band to a camera direction WITH the
/// screen-y → camera-y flip: top → `+Y`, bottom → `-Y`, left → `-X`, right → `+X`, a
/// corner → a diagonal, and the centre → `ZERO`. Relations, not pinned magnitudes.
#[test]
fn mouse_edge_dir_maps_each_edge_with_the_y_flip() {
    // TOP band (small screen y) pans the camera UP (+Y) — the explicit flip.
    let top = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, 5.0), SIZE, EDGE);
    assert!(
        top.y > 0.0,
        "cursor near the TOP must pan the camera UP (+Y)"
    );
    assert_eq!(
        top.x.to_bits(),
        0.0_f32.to_bits(),
        "a centred-x top has no x pan"
    );

    // BOTTOM band (large screen y) pans the camera DOWN (-Y).
    let bottom = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, SIZE.y - 5.0), SIZE, EDGE);
    assert!(
        bottom.y < 0.0,
        "cursor near the BOTTOM must pan the camera DOWN (-Y)"
    );

    // LEFT band → -X.
    let left = mouse_edge_dir(Vec2::new(5.0, SIZE.y * 0.5), SIZE, EDGE);
    assert!(left.x < 0.0, "cursor near the LEFT must pan -X");
    assert_eq!(
        left.y.to_bits(),
        0.0_f32.to_bits(),
        "a centred-y left has no y pan"
    );

    // RIGHT band → +X.
    let right = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, SIZE.y * 0.5), SIZE, EDGE);
    assert!(right.x > 0.0, "cursor near the RIGHT must pan +X");

    // CORNER (top-right) → a diagonal: +X and +Y.
    let corner = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, 5.0), SIZE, EDGE);
    assert!(
        corner.x > 0.0 && corner.y > 0.0,
        "the top-right corner must pan diagonally (+X, +Y)"
    );

    // CENTRE (no band) → ZERO.
    assert_eq!(
        mouse_edge_dir(SIZE * 0.5, SIZE, EDGE),
        Vec2::ZERO,
        "a cursor in the centre (no edge band) contributes no pan",
    );
}

/// AC2 — `keyboard_pan_dir` maps W/A/S/D to camera axes, opposite keys cancel, and
/// (via the system) arrows alias WASD. W → `+Y`, S → `-Y`, A → `-X`, D → `+X`,
/// W+D → up-right, W+S → `ZERO`.
#[test]
fn keyboard_pan_dir_combines_keys_and_cancels_opposites() {
    // Single keys (W / S / A / D).
    assert_eq!(
        keyboard_pan_dir(true, false, false, false),
        Vec2::new(0.0, 1.0),
        "W → +Y",
    );
    assert_eq!(
        keyboard_pan_dir(false, true, false, false),
        Vec2::new(0.0, -1.0),
        "S → -Y",
    );
    assert_eq!(
        keyboard_pan_dir(false, false, true, false),
        Vec2::new(-1.0, 0.0),
        "A → -X",
    );
    assert_eq!(
        keyboard_pan_dir(false, false, false, true),
        Vec2::new(1.0, 0.0),
        "D → +X",
    );

    // Combination: W+D → up-right.
    assert_eq!(
        keyboard_pan_dir(true, false, false, true),
        Vec2::new(1.0, 1.0),
        "W+D → up-right (+X, +Y)",
    );

    // Opposite keys cancel.
    assert_eq!(
        keyboard_pan_dir(true, true, false, false),
        Vec2::ZERO,
        "W+S cancel → ZERO",
    );
    assert_eq!(
        keyboard_pan_dir(false, false, true, true),
        Vec2::ZERO,
        "A+D cancel → ZERO",
    );
}

/// AC3 — `stick_pan_dir` zeroes a sub-deadzone stick and passes a clearly-past stick
/// through with the camera-y orientation preserved (stick up → +Y).
#[test]
fn stick_pan_dir_respects_the_deadzone() {
    let deadzone = StickDeadzone::new(0.15);

    // A tiny resting drift below the deadzone → ZERO.
    assert_eq!(
        stick_pan_dir(Vec2::new(0.05, -0.05), deadzone),
        Vec2::ZERO,
        "a sub-deadzone stick contributes no pan",
    );

    // A clear push past the deadzone passes through, magnitude preserved, stick-up = +Y.
    let pushed = Vec2::new(0.0, 0.8);
    assert_eq!(
        stick_pan_dir(pushed, deadzone),
        pushed,
        "a clearly-past stick passes through unchanged (stick up → camera +Y)",
    );
    assert!(
        stick_pan_dir(Vec2::new(0.0, 0.8), deadzone).y > 0.0,
        "stick UP must map to camera +Y",
    );
}

/// AC4 — `pan_velocity` scales a unit direction by speed, returns ZERO for no input,
/// and the diagonal-not-faster rule holds for the keyboard combination (a normalised
/// diagonal is no faster than a cardinal).
#[test]
fn pan_velocity_scales_and_diagonal_is_not_faster() {
    let speed = PanSpeed::new(100.0);

    // ZERO direction → ZERO velocity.
    assert_eq!(
        pan_velocity(Vec2::ZERO, speed),
        Vec2::ZERO,
        "no input → no velocity (no drift)",
    );

    // A unit cardinal → speed-scaled.
    let cardinal = pan_velocity(Vec2::new(0.0, 1.0), speed);
    assert_eq!(
        cardinal,
        Vec2::new(0.0, 100.0),
        "a unit direction scales to exactly `speed` world-units/sec",
    );

    // Diagonal-not-faster: the (normalised) diagonal speed equals the cardinal speed.
    let diagonal = pan_velocity(Vec2::new(1.0, 1.0), speed);
    let diag_speed = diagonal.length();
    let card_speed = cardinal.length();
    assert!(
        (diag_speed - card_speed).abs() < 1e-3,
        "a diagonal keyboard combo must not be faster than a cardinal (got diag {diag_speed}, \
         cardinal {card_speed})",
    );

    // A sub-unit analog magnitude scales speed DOWN (a gentle stick push pans gently).
    let gentle = pan_velocity(Vec2::new(0.0, 0.5), speed);
    assert!(
        gentle.length() < card_speed,
        "a sub-unit analog stick magnitude keeps its sub-unit scale (pans slower)",
    );
}

// -----------------------------------------------------------------------------
// GTW-271 — edge-pan keys off the MAP VIEWPORT rect, not the window.
// -----------------------------------------------------------------------------

/// A test map-viewport rect: a 600x400 central region inset 100px from the left and 50px
/// from the top of an 800x600 window (the status panel on the left, a minimal top inset).
const VIEWPORT: Rect = Rect {
    min: Vec2::new(100.0, 50.0),
    max: Vec2::new(700.0, 450.0),
};

/// GTW-271 AC4 — `viewport_edge_dir` pans ONLY when the cursor is INSIDE the map viewport
/// rect AND within the edge band; a cursor OUTSIDE the rect (in a UI margin / over a panel)
/// → `Vec2::ZERO` (no pan). Inside-near-an-edge it reproduces the window-edge logic measured
/// from the VIEWPORT edges (with the screen-y → camera-y flip).
///
/// Pin-discriminating: it proves a cursor in the LEFT MARGIN (left of the viewport min, where
/// the status panel sits) does NOT pan even though it is near the WINDOW's left edge — the
/// behaviour the old window-relative `mouse_edge_dir` got wrong (it would have panned -X under
/// the panel). And a cursor just INSIDE the viewport's left edge DOES pan -X. Pure, no `App`.
#[test]
fn viewport_edge_dir_pans_only_inside_the_map_rect() {
    // A cursor in the LEFT MARGIN (x < viewport.min.x — over the status panel): no pan, even
    // though it is hard against the WINDOW's left edge.
    let in_margin = viewport_edge_dir(Vec2::new(10.0, 250.0), VIEWPORT, EDGE);
    assert_eq!(
        in_margin,
        Vec2::ZERO,
        "a cursor in the left margin (outside the map viewport) must NOT pan",
    );

    // A cursor BELOW the viewport (y > viewport.max.y — over the action-bar margin): no pan.
    let below = viewport_edge_dir(Vec2::new(400.0, 580.0), VIEWPORT, EDGE);
    assert_eq!(
        below,
        Vec2::ZERO,
        "a cursor below the map viewport (over the action-bar margin) must NOT pan",
    );

    // A cursor just INSIDE the viewport's LEFT edge (within the band of viewport.min.x) → -X.
    let near_left = viewport_edge_dir(Vec2::new(VIEWPORT.min.x + 5.0, 250.0), VIEWPORT, EDGE);
    assert!(
        near_left.x < 0.0,
        "a cursor just inside the viewport's left edge must pan -X (got {near_left:?})",
    );

    // A cursor just INSIDE the viewport's TOP edge → +Y (the screen-y → camera-y flip,
    // measured from the VIEWPORT top, not the window top).
    let near_top = viewport_edge_dir(Vec2::new(400.0, VIEWPORT.min.y + 5.0), VIEWPORT, EDGE);
    assert!(
        near_top.y > 0.0,
        "a cursor just inside the viewport's top edge must pan UP (+Y) (got {near_top:?})",
    );

    // A cursor in the CENTRE of the viewport (no edge band) → ZERO.
    let centre = viewport_edge_dir(VIEWPORT.min + VIEWPORT.size() * 0.5, VIEWPORT, EDGE);
    assert_eq!(
        centre,
        Vec2::ZERO,
        "a cursor in the centre of the map viewport contributes no pan",
    );
}
