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


const AC6_WIN_W: u32 = 1600;
const AC6_WIN_H: u32 = 1200;
const AC6_SCALE: f32 = 2.0;
const AC6_VIEWPORT_PHYS: UVec2 = UVec2::new(800, 600);

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
    app.world_mut().insert_resource(margin_tuning(MARGIN));

    let mut window = Window::default();
    window
        .resolution
        .set_physical_resolution(AC6_WIN_W, AC6_WIN_H);
    window.resolution.set_scale_factor(AC6_SCALE);
    app.world_mut().spawn((window, PrimaryWindow));

    let projection = Projection::Orthographic(OrthographicProjection {
        scale: 1.0,
        area: Rect::from_corners(Vec2::ZERO, Vec2::ZERO),
        ..OrthographicProjection::default_2d()
    });
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
    let (hard_min, hard_max) = battlefield_bounds();
    let min = hard_min - Vec2::splat(MARGIN);
    let max = hard_max + Vec2::splat(MARGIN);

    let viewport_half = AC6_VIEWPORT_PHYS.as_vec2() / AC6_SCALE * 0.5;
    let window_logical = Vec2::new(AC6_WIN_W as f32, AC6_WIN_H as f32) / AC6_SCALE;
    let window_half = window_logical * 0.5;

    assert!(
        viewport_half.x < window_half.x && viewport_half.y < window_half.y,
        "fixture invariant: the sub-rect viewport must derive a smaller half-extent than the window",
    );

    let expected_viewport = clamp_camera(far_outside, viewport_half, min, max);
    assert_eq!(
        after, expected_viewport,
        "the fallback clamp must use the VIEWPORT-derived half-extent (physical size / scale factor)",
    );

    let reverted_window = clamp_camera(far_outside, window_half, min, max);
    assert_ne!(
        after, reverted_window,
        "the fallback must NOT clamp with the full-window half-extent once a sub-rect viewport is set",
    );
}
