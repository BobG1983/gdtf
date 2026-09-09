use bevy::{
    MinimalPlugins,
    app::{App, Update},
    camera::{OrthographicProjection, Projection},
    ecs::schedule::SystemCondition,
    math::Vec2,
    prelude::{Camera2d, IntoScheduleConfigs, Transform, resource_exists},
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_presenter::{WorldCamera, clamp_camera_to_bounds};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Faction},
};

use super::harness::*;

fn field_centre_y() -> f32 {
    let (min, max) = battlefield_bounds();
    f32::midpoint(min.y, max.y)
}

#[test]
fn clamp_pulls_out_of_bounds_camera_back_inside() {
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

    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(VIEWPORT.x as u32, VIEWPORT.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));

    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(VIEWPORT.x, VIEWPORT.y);
    let far_outside = Vec2::new(-100_000.0, 100_000.0);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        projection,
        Transform::from_xyz(far_outside.x, far_outside.y, 0.0),
    ));

    let before = camera_xy(&mut app);
    assert_eq!(
        before, far_outside,
        "precondition: the camera starts far out of bounds"
    );

    app.update();

    let after = camera_xy(&mut app);
    assert_ne!(
        after, far_outside,
        "the clamp must MOVE a far-out-of-bounds camera",
    );

    let (hard_min, hard_max) = battlefield_bounds();
    let relaxed_min = hard_min - Vec2::splat(MARGIN);
    let relaxed_max = hard_max + Vec2::splat(MARGIN);
    let half = VIEWPORT * 0.5;
    assert!(
        after.x - half.x >= relaxed_min.x - f32::EPSILON
            && after.x + half.x <= relaxed_max.x + f32::EPSILON,
        "after the clamp the viewport must stay within the RELAXED (bounds + margin) x bounds",
    );
    assert!(
        after.y - half.y >= relaxed_min.y - f32::EPSILON
            && after.y + half.y <= relaxed_max.y + f32::EPSILON,
        "after the clamp the viewport must stay within the RELAXED (bounds + margin) y bounds",
    );
}

fn relaxed_clamp_app(at: Vec2) -> App {
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
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(VIEWPORT.x as u32, VIEWPORT.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(VIEWPORT.x, VIEWPORT.y);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        projection,
        Transform::from_xyz(at.x, at.y, 0.0),
    ));
    app
}

#[test]
fn relaxed_clamp_pulls_back_to_bounds_plus_margin() {
    let (_hard_min, hard_max) = battlefield_bounds();
    let half = VIEWPORT * 0.5;
    let relaxed_edge_x = (hard_max.x + MARGIN) - half.x;
    let hard_edge_x = hard_max.x - half.x;
    let start = Vec2::new(relaxed_edge_x + 10_000.0, field_centre_y());

    let mut app = relaxed_clamp_app(start);
    app.update();
    let after = camera_xy(&mut app);

    assert!(
        (after.x - relaxed_edge_x).abs() < f32::EPSILON,
        "a camera beyond bounds+margin must be pulled back to the bounds+margin edge ({relaxed_edge_x}), got {}",
        after.x,
    );
    assert!(
        (after.x - hard_edge_x).abs() > HALF_MARGIN,
        "the pull-back edge must be the RELAXED bounds+margin, NOT the old hard bounds ({hard_edge_x})",
    );
}

#[test]
fn within_margin_camera_is_not_clamped() {
    let (_hard_min, hard_max) = battlefield_bounds();
    let half = VIEWPORT * 0.5;
    let hard_edge_x = hard_max.x - half.x;
    let within = Vec2::new(hard_edge_x + HALF_MARGIN, field_centre_y());

    assert!(
        within.x > hard_edge_x,
        "fixture invariant: the within-margin position must sit PAST the old hard-bounds edge",
    );

    let mut app = relaxed_clamp_app(within);
    app.update();
    let after = camera_xy(&mut app);

    assert!(
        (after.x - within.x).abs() < f32::EPSILON && (after.y - within.y).abs() < f32::EPSILON,
        "a camera within bounds+margin must NOT be clamped — it stays where it panned ({within}), got {after}",
    );
}
