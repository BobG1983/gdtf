//! GTW-271 viewport-aware fallback half-extent (AC6).

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    camera::{OrthographicProjection, Projection, Viewport},
    ecs::schedule::SystemCondition,
    math::{Rect, UVec2, Vec2},
    prelude::{Camera, Camera2d, IntoScheduleConfigs, Transform, resource_exists},
    window::{PrimaryWindow, Window},
};
use gdtf_battle_presenter::{WorldCamera, clamp_camera, clamp_camera_to_bounds};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Faction},
};

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-271 AC6 — the pre-`camera_system` fallback half-extent is VIEWPORT-aware.
// ---------------------------------------------------------------------------------

/// The AC6 window's physical width (the FULL surface — what the OLD full-window fallback
/// would use for the half-extent).
const AC6_WIN_W: u32 = 1600;
/// The AC6 window's physical height.
const AC6_WIN_H: u32 = 1200;
/// The AC6 window's scale factor — `> 1.0` so the physical→logical division in the fallback
/// is actually exercised (a revert to bare `physical_size`, dropping `/ scale_factor`, also
/// flips the asserted half-extent, not only a revert to the full window).
const AC6_SCALE: f32 = 2.0;
/// The AC6 map sub-rect's physical size — SMALLER than the window, so its derived half-extent
/// is smaller than the full-window fallback's and the two clamp results differ.
const AC6_VIEWPORT_PHYS: UVec2 = UVec2::new(800, 600);

/// GTW-271 AC6 — `clamp_camera_to_bounds` clamps using the VIEWPORT-derived half-extent (not
/// the full window) when the orthographic `area` is not yet computed (the pre-`camera_system`
/// fallback) and a sub-rect `Camera.viewport` is set.
///
/// Pin-discriminating: the camera starts far out of bounds with a DEGENERATE orthographic
/// `area` (half-size 0 → the steady-state `area_half > 0` branch is skipped and the fallback
/// runs) and an explicit sub-rect `Camera.viewport`. The assertion compares the clamped
/// position against `clamp_camera(.., viewport_half, ..)` (the SMALLER, viewport-derived
/// half-extent = viewport physical size / scale factor * 0.5) AND asserts it is NOT
/// `clamp_camera(.., window_half, ..)` (the larger, full-window half-extent the OLD fallback
/// produced). Reverting the `Some(viewport) => ..` fallback branch to `window.size()` makes
/// the system use `window_half`, flipping BOTH assertions. The bounds + both half-extents are
/// derived from `battlefield_bounds` + the (window, scale, viewport) sizes, so no scene
/// magnitude is pinned — only the viewport-vs-window RELATION.
#[test]
fn clamp_uses_the_viewport_half_extent_in_the_fallback() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        clamp_camera_to_bounds
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>)),
    );

    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));
    // The same KNOWN off-level margin: the expected `clamp_camera` calls below relax the bounds
    // by it, so this test pins the VIEWPORT-vs-window half-extent relation through the relaxed clamp.
    app.world_mut().insert_resource(margin_tuning(MARGIN));

    // A primary window at a KNOWN physical size + scale factor (> 1.0), so the fallback's
    // physical→logical conversion (`physical_size / scale_factor`) is exercised.
    let mut window = Window::default();
    window
        .resolution
        .set_physical_resolution(AC6_WIN_W, AC6_WIN_H);
    window.resolution.set_scale_factor(AC6_SCALE);
    app.world_mut().spawn((window, PrimaryWindow));

    // A WorldCamera whose orthographic `area` is DEGENERATE (half-size 0), so the steady-state
    // `area_half > 0` branch is skipped under MinimalPlugins (no `camera_system` to compute the
    // area) and the GTW-271 fallback runs. `scale = 1.0` keeps the half-extent arithmetic clean.
    let projection = Projection::Orthographic(OrthographicProjection {
        scale: 1.0,
        area: Rect::from_corners(Vec2::ZERO, Vec2::ZERO),
        ..OrthographicProjection::default_2d()
    });
    // The sub-rect map viewport (physical px) the app would set in AC1. Origin is irrelevant to
    // the half-extent (only the SIZE feeds it).
    let camera = Camera {
        viewport: Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size:     AC6_VIEWPORT_PHYS,
            depth:             0.0..1.0,
        }),
        ..Default::default()
    };
    let far_outside = Vec2::new(-100_000.0, -100_000.0);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        camera,
        projection,
        Transform::from_xyz(far_outside.x, far_outside.y, 0.0),
    ));

    app.update();

    let after = camera_xy(&mut app);
    // GTW-381: the clamp relaxes the bounds by the inserted margin, so the expected `clamp_camera`
    // calls relax the bounds the same way (the half-extent relation under test is unchanged).
    let (hard_min, hard_max) = battlefield_bounds();
    let min = hard_min - Vec2::splat(MARGIN);
    let max = hard_max + Vec2::splat(MARGIN);

    // The half-extents the two fallback branches produce (scale = 1.0): the viewport branch
    // divides by the scale factor, the full-window branch uses the logical window size.
    let viewport_half = AC6_VIEWPORT_PHYS.as_vec2() / AC6_SCALE * 0.5;
    let window_logical = Vec2::new(AC6_WIN_W as f32, AC6_WIN_H as f32) / AC6_SCALE;
    let window_half = window_logical * 0.5;

    // Sanity: the viewport half-extent really is SMALLER, so the two clamp results differ.
    assert!(
        viewport_half.x < window_half.x && viewport_half.y < window_half.y,
        "fixture invariant: the sub-rect viewport must derive a smaller half-extent than the window",
    );

    // The clamp must use the VIEWPORT-derived half-extent (against the relaxed bounds).
    let expected_viewport = clamp_camera(far_outside, viewport_half, min, max);
    assert_eq!(
        after, expected_viewport,
        "the fallback clamp must use the VIEWPORT-derived half-extent (physical size / scale factor)",
    );

    // And NOT the full-window half-extent (what the OLD fallback / a revert produces) — this is
    // the AC6 pin: reverting `Some(viewport) => ..` to `window.size()` flips this assertion.
    let reverted_window = clamp_camera(far_outside, window_half, min, max);
    assert_ne!(
        after, reverted_window,
        "the fallback must NOT clamp with the full-window half-extent once a sub-rect viewport is set",
    );
}
