use bevy::prelude::*;

use super::super::pan::{
    EdgeBandPx, PanAxis, PanSpeed, StickDeadzone, keyboard_pan_dir, mouse_edge_dir, pan_velocity,
    stick_pan_dir, viewport_edge_dir,
};

// -----------------------------------------------------------------------------

const SIZE: Vec2 = Vec2::new(800.0, 600.0);
const EDGE: EdgeBandPx = EdgeBandPx::new(20.0);

#[test]
fn mouse_edge_dir_maps_each_edge_with_the_y_flip() {
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

    let bottom = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, SIZE.y - 5.0), SIZE, EDGE);
    assert!(
        bottom.y < 0.0,
        "cursor near the BOTTOM must pan the camera DOWN (-Y)"
    );

    let left = mouse_edge_dir(Vec2::new(5.0, SIZE.y * 0.5), SIZE, EDGE);
    assert!(left.x < 0.0, "cursor near the LEFT must pan -X");
    assert_eq!(
        left.y.to_bits(),
        0.0_f32.to_bits(),
        "a centred-y left has no y pan"
    );

    let right = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, SIZE.y * 0.5), SIZE, EDGE);
    assert!(right.x > 0.0, "cursor near the RIGHT must pan +X");

    let corner = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, 5.0), SIZE, EDGE);
    assert!(
        corner.x > 0.0 && corner.y > 0.0,
        "the top-right corner must pan diagonally (+X, +Y)"
    );

    assert_eq!(
        mouse_edge_dir(SIZE * 0.5, SIZE, EDGE),
        Vec2::ZERO,
        "a cursor in the centre (no edge band) contributes no pan",
    );
}

#[test]
fn keyboard_pan_dir_combines_axes_and_cancels_opposites() {
    assert_eq!(
        keyboard_pan_dir(PanAxis::Positive, PanAxis::Still),
        Vec2::new(0.0, 1.0),
        "W → +Y",
    );
    assert_eq!(
        keyboard_pan_dir(PanAxis::Negative, PanAxis::Still),
        Vec2::new(0.0, -1.0),
        "S → -Y",
    );
    assert_eq!(
        keyboard_pan_dir(PanAxis::Still, PanAxis::Negative),
        Vec2::new(-1.0, 0.0),
        "A → -X",
    );
    assert_eq!(
        keyboard_pan_dir(PanAxis::Still, PanAxis::Positive),
        Vec2::new(1.0, 0.0),
        "D → +X",
    );

    assert_eq!(
        keyboard_pan_dir(PanAxis::Positive, PanAxis::Positive),
        Vec2::new(1.0, 1.0),
        "W+D → up-right (+X, +Y)",
    );

    assert_eq!(
        keyboard_pan_dir(PanAxis::from_keys(true, true), PanAxis::Still),
        Vec2::ZERO,
        "W+S cancel → ZERO",
    );
    assert_eq!(
        keyboard_pan_dir(PanAxis::Still, PanAxis::from_keys(true, true)),
        Vec2::ZERO,
        "A+D cancel → ZERO",
    );
}

#[test]
fn pan_axis_resolves_each_key_pair() {
    assert_eq!(
        PanAxis::from_keys(true, false),
        PanAxis::Positive,
        "the positive key alone pushes the axis positive",
    );
    assert_eq!(
        PanAxis::from_keys(false, true),
        PanAxis::Negative,
        "the negative key alone pushes the axis negative",
    );
    assert_eq!(
        PanAxis::from_keys(true, true),
        PanAxis::Still,
        "both keys held cancel to Still",
    );
    assert_eq!(
        PanAxis::from_keys(false, false),
        PanAxis::Still,
        "neither key held is Still",
    );
    assert_eq!(
        PanAxis::default(),
        PanAxis::Still,
        "an unpushed axis defaults to Still",
    );
}

#[test]
fn stick_pan_dir_respects_the_deadzone() {
    let deadzone = StickDeadzone::new(0.15);

    assert_eq!(
        stick_pan_dir(Vec2::new(0.05, -0.05), deadzone),
        Vec2::ZERO,
        "a sub-deadzone stick contributes no pan",
    );

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

#[test]
fn pan_velocity_scales_and_diagonal_is_not_faster() {
    let speed = PanSpeed::new(100.0);

    assert_eq!(
        pan_velocity(Vec2::ZERO, speed),
        Vec2::ZERO,
        "no input → no velocity (no drift)",
    );

    let cardinal = pan_velocity(Vec2::new(0.0, 1.0), speed);
    assert_eq!(
        cardinal,
        Vec2::new(0.0, 100.0),
        "a unit direction scales to exactly `speed` world-units/sec",
    );

    let diagonal = pan_velocity(Vec2::new(1.0, 1.0), speed);
    let diag_speed = diagonal.length();
    let card_speed = cardinal.length();
    assert!(
        (diag_speed - card_speed).abs() < 1e-3,
        "a diagonal keyboard combo must not be faster than a cardinal (got diag {diag_speed}, \
         cardinal {card_speed})",
    );

    let gentle = pan_velocity(Vec2::new(0.0, 0.5), speed);
    assert!(
        gentle.length() < card_speed,
        "a sub-unit analog stick magnitude keeps its sub-unit scale (pans slower)",
    );
}

const VIEWPORT: Rect = Rect {
    min: Vec2::new(100.0, 50.0),
    max: Vec2::new(700.0, 450.0),
};

#[test]
fn viewport_edge_dir_pans_only_inside_the_map_rect() {
    let in_margin = viewport_edge_dir(Vec2::new(10.0, 250.0), VIEWPORT, EDGE);
    assert_eq!(
        in_margin,
        Vec2::ZERO,
        "a cursor in the left margin (outside the map viewport) must NOT pan",
    );

    let below = viewport_edge_dir(Vec2::new(400.0, 580.0), VIEWPORT, EDGE);
    assert_eq!(
        below,
        Vec2::ZERO,
        "a cursor below the map viewport (over the action-bar margin) must NOT pan",
    );

    let near_left = viewport_edge_dir(Vec2::new(VIEWPORT.min.x + 5.0, 250.0), VIEWPORT, EDGE);
    assert!(
        near_left.x < 0.0,
        "a cursor just inside the viewport's left edge must pan -X (got {near_left:?})",
    );

    let near_top = viewport_edge_dir(Vec2::new(400.0, VIEWPORT.min.y + 5.0), VIEWPORT, EDGE);
    assert!(
        near_top.y > 0.0,
        "a cursor just inside the viewport's top edge must pan UP (+Y) (got {near_top:?})",
    );

    let centre = viewport_edge_dir(VIEWPORT.min + VIEWPORT.size() * 0.5, VIEWPORT, EDGE);
    assert_eq!(
        centre,
        Vec2::ZERO,
        "a cursor in the centre of the map viewport contributes no pan",
    );
}
