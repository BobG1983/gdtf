//! Tests for the pure software-cursor step helper (relocated from `gamepad.rs`, GTW-201).

use bevy::prelude::Vec2;

use crate::gamepad::cursor::{CursorSpeed, move_cursor};

/// A test window size for the cursor clamp.
const WINDOW: Vec2 = Vec2::new(800.0, 600.0);
/// A test cursor speed (px/sec).
const SPEED: CursorSpeed = CursorSpeed::new(100.0);

/// AC1 — `move_cursor` moves with the stick-y → screen-y FLIP, scales by speed·dt, and a zero
/// stick leaves the position unchanged.
#[test]
fn move_cursor_flips_y_and_scales() {
    let start = Vec2::new(400.0, 300.0); // window centre.

    // Stick UP (+y) moves the cursor UP on screen (screen-y DECREASES) — the flip.
    let up = move_cursor(start, Vec2::new(0.0, 1.0), SPEED, 0.5, WINDOW);
    assert!(
        up.y < start.y,
        "stick UP (+y) must move the cursor UP on screen (screen-y decreases): {} -> {}",
        start.y,
        up.y,
    );
    assert_eq!(
        up.x.to_bits(),
        start.x.to_bits(),
        "a pure-up stick has no x movement",
    );

    // Stick RIGHT (+x) moves the cursor RIGHT (+x), no flip.
    let right = move_cursor(start, Vec2::new(1.0, 0.0), SPEED, 0.5, WINDOW);
    assert!(
        right.x > start.x,
        "stick RIGHT (+x) must move the cursor RIGHT (+x): {} -> {}",
        start.x,
        right.x,
    );

    // speed·dt scales the move: 100 px/s × 0.5 s = 50 px right.
    assert_eq!(
        right.x.to_bits(),
        (start.x + 50.0).to_bits(),
        "the move scales by speed·dt (100 × 0.5 = 50 px)",
    );

    // Zero stick → unchanged (clamped, but the centre is already inside the window).
    assert_eq!(
        move_cursor(start, Vec2::ZERO, SPEED, 0.5, WINDOW),
        start,
        "a zero stick leaves the cursor where it is",
    );
}

/// AC1 — `move_cursor` clamps the result componentwise into `[ZERO, window]`, so the cursor can
/// never leave the window in any direction.
#[test]
fn move_cursor_clamps_inside_the_window() {
    // Push HARD up-left from near the top-left corner: a huge speed·dt would overshoot past (0,0),
    // but the clamp pins it at ZERO.
    let near_corner = Vec2::new(10.0, 10.0);
    let pinned_low = move_cursor(
        near_corner,
        Vec2::new(-1.0, 1.0),
        CursorSpeed::new(10_000.0),
        1.0,
        WINDOW,
    );
    assert_eq!(
        pinned_low,
        Vec2::ZERO,
        "a hard up-left push clamps to the top-left corner (ZERO), never past it",
    );

    // Push HARD down-right from near the far corner: clamps at `window`.
    let near_far = WINDOW - Vec2::new(10.0, 10.0);
    let pinned_high = move_cursor(
        near_far,
        Vec2::new(1.0, -1.0),
        CursorSpeed::new(10_000.0),
        1.0,
        WINDOW,
    );
    assert_eq!(
        pinned_high, WINDOW,
        "a hard down-right push clamps to the window extent, never past it",
    );
}
