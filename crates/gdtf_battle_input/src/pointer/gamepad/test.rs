use bevy::prelude::Vec2;

use crate::gamepad::cursor::{CursorSpeed, move_cursor};

const WINDOW: Vec2 = Vec2::new(800.0, 600.0);
const SPEED: CursorSpeed = CursorSpeed::new(100.0);

#[test]
fn move_cursor_flips_y_and_scales() {
    let start = Vec2::new(400.0, 300.0); 

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

    let right = move_cursor(start, Vec2::new(1.0, 0.0), SPEED, 0.5, WINDOW);
    assert!(
        right.x > start.x,
        "stick RIGHT (+x) must move the cursor RIGHT (+x): {} -> {}",
        start.x,
        right.x,
    );

    assert_eq!(
        right.x.to_bits(),
        (start.x + 50.0).to_bits(),
        "the move scales by speed·dt (100 × 0.5 = 50 px)",
    );

    assert_eq!(
        move_cursor(start, Vec2::ZERO, SPEED, 0.5, WINDOW),
        start,
        "a zero stick leaves the cursor where it is",
    );
}

#[test]
fn move_cursor_clamps_inside_the_window() {
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
