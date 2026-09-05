//! Camera pan: WASD moves the world camera and stays inside battlefield bounds.
use bevy::{
    ecs::prelude::With,
    input::{ButtonInput, keyboard::KeyCode},
    math::Vec2,
    transform::components::Transform,
};
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    battle::{BattleInProgress, PlayerFaction},
    metric::{Cell, Level},
};
use gdtf_game::test_support::BattleAppBuilder;

const PAN_UPDATES: u32 = 12;

fn live_battle_app() -> bevy::app::App {
    BattleAppBuilder::new().build()
}

fn camera_xy(app: &mut bevy::app::App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

fn set_camera_xy(app: &mut bevy::app::App, to: Vec2) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&mut Transform, With<WorldCamera>>();
    for mut transform in query.iter_mut(world) {
        transform.translation.x = to.x;
        transform.translation.y = to.y;
    }
}

fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::occupancy::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::occupancy::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        gdtf_battle_presenter::cell_to_world(Cell::new(0, 0), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(w, 0), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(0, h), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(w, h), Level::new(0)),
    ];
    let mut min = Vec2::new(corners[0].x, corners[0].y);
    let mut max = min;
    for c in corners {
        let p = Vec2::new(c.x, c.y);
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

fn battlefield_centre() -> Vec2 {
    let (min, max) = battlefield_bounds();
    (min + max) * 0.5
}

fn hold_key_for_pan(app: &mut bevy::app::App, key_code: KeyCode) {
    for _ in 0..PAN_UPDATES {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key_code);
        app.update();
    }
}

#[test]
fn keyboard_w_pans_camera_up_within_bounds() {
    let mut app = live_battle_app();

    assert!(
        app.world().get_resource::<BattleInProgress>().is_some()
            && app.world().get_resource::<PlayerFaction>().is_some(),
        "precondition: the live battle's BattleInProgress + PlayerFaction gate the camera systems",
    );

    let centre = battlefield_centre();
    set_camera_xy(&mut app, centre);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let baseline = camera_xy(&mut app);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let no_press = camera_xy(&mut app);
    assert_eq!(
        no_press, baseline,
        "with no pan key pressed, the camera must NOT move",
    );

    hold_key_for_pan(&mut app, KeyCode::KeyW);
    let after_w = camera_xy(&mut app);
    assert!(
        after_w.y > baseline.y,
        "holding W must pan the WorldCamera UP (+Y): baseline y {} -> after {}",
        baseline.y,
        after_w.y,
    );

    let (min, max) = battlefield_bounds();
    assert!(
        after_w.y <= max.y + f32::EPSILON && after_w.y >= min.y - f32::EPSILON,
        "after the pan the camera y ({}) must stay within the battlefield y bounds [{}, {}] — the \
         clamp ran after the pan",
        after_w.y,
        min.y,
        max.y,
    );
    assert!(
        after_w.x <= max.x + f32::EPSILON && after_w.x >= min.x - f32::EPSILON,
        "the camera x must stay within the battlefield x bounds",
    );
}
