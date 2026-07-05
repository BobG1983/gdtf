//! Tests of the framing-geometry pure helpers (mirrors `framing.rs`).

use bevy::prelude::*;

use super::super::framing::{camera_focus, clamp_camera};

/// AC1 — `camera_focus` is the centroid (mean) of the supplied points, and `None`
/// for an empty iterator (so the framing leaves the camera be when no player gangers
/// exist). Pure, no `App`.
#[test]
fn camera_focus_is_the_centroid_or_none() {
    // Three points -> their mean.
    let three = [
        Vec2::new(0.0, 0.0),
        Vec2::new(6.0, 0.0),
        Vec2::new(0.0, 9.0),
    ];
    let mean = camera_focus(three);
    assert!(mean.is_some(), "three points must have a centroid");
    let Some(mean) = mean else { return };
    assert_eq!(
        mean.x.to_bits(),
        2.0_f32.to_bits(),
        "mean x = (0+6+0)/3 = 2"
    );
    assert_eq!(
        mean.y.to_bits(),
        3.0_f32.to_bits(),
        "mean y = (0+0+9)/3 = 3"
    );

    // One point -> that point.
    let one = camera_focus([Vec2::new(4.0, -7.0)]);
    assert!(one.is_some(), "one point must have a centroid");
    let Some(one) = one else { return };
    assert_eq!(
        one,
        Vec2::new(4.0, -7.0),
        "a single point is its own centroid"
    );

    // Empty -> None.
    assert!(
        camera_focus(std::iter::empty::<Vec2>()).is_none(),
        "an empty iterator has no centroid",
    );
}

/// AC2 — `clamp_camera` keeps the viewport inside the bounds and centres on the map
/// midpoint when the map is smaller than the viewport. Relations, not pinned scene
/// magnitudes. Pure, no `App`.
#[test]
fn clamp_camera_keeps_viewport_inside_and_centres_when_smaller() {
    let half = Vec2::new(10.0, 10.0);
    let world_min = Vec2::new(0.0, 0.0);
    let world_max = Vec2::new(100.0, 100.0);

    // Pushed past world_max -> clamped to world_max - half on that axis.
    let past_max = clamp_camera(Vec2::new(1000.0, 1000.0), half, world_min, world_max);
    assert_eq!(
        past_max,
        world_max - half,
        "a translation past world_max clamps to world_max - half",
    );

    // Pushed past world_min -> clamped to world_min + half on that axis.
    let past_min = clamp_camera(Vec2::new(-1000.0, -1000.0), half, world_min, world_max);
    assert_eq!(
        past_min,
        world_min + half,
        "a translation past world_min clamps to world_min + half",
    );

    // A within-bounds translation is unchanged.
    let inside = Vec2::new(50.0, 40.0);
    assert_eq!(
        clamp_camera(inside, half, world_min, world_max),
        inside,
        "a translation already inside the bounds is left unchanged",
    );

    // Map SMALLER than the viewport on both axes (2*half > span) -> the map midpoint.
    let big_half = Vec2::new(80.0, 80.0); // 2*80 = 160 > 100 span on each axis.
    let centred = clamp_camera(Vec2::new(1000.0, -1000.0), big_half, world_min, world_max);
    let midpoint = (world_min + world_max) * 0.5;
    assert_eq!(
        centred, midpoint,
        "when the map is narrower than the viewport, the camera centres on the midpoint",
    );
}
