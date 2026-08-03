use core::time::Duration;

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    ecs::schedule::SystemCondition,
    math::Vec2,
    prelude::{Camera2d, IntoScheduleConfigs, Transform, With, resource_exists},
    time::TimeUpdateStrategy,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_presenter::{
    GamepadCursorMoved, WorldCamera, cell_to_world, clamp_camera_to_bounds,
    pan_camera_on_gamepad_cursor_edge,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Cell, Faction, Level},
};

const WINDOW: Vec2 = Vec2::new(800.0, 600.0);
const STEP: Duration = Duration::from_millis(250);
const PLAYER_GANG: u8 = 0;

fn edge_pan_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP))
        .add_message::<GamepadCursorMoved>()
        .add_systems(
            Update,
            (
                pan_camera_on_gamepad_cursor_edge,
                clamp_camera_to_bounds.after(pan_camera_on_gamepad_cursor_edge),
            )
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>),
                ),
        );
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        Transform::from_translation(battlefield_centre().extend(0.0)),
    ));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(WINDOW.x as u32, WINDOW.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));
    app
}

fn camera_xy(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::occupancy::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::occupancy::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        cell_to_world(Cell::new(0, 0), Level::new(0)),
        cell_to_world(Cell::new(w, 0), Level::new(0)),
        cell_to_world(Cell::new(0, h), Level::new(0)),
        cell_to_world(Cell::new(w, h), Level::new(0)),
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

fn send_cursor(app: &mut App, pos: Vec2) {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<GamepadCursorMoved>>()
        .write(GamepadCursorMoved::new(pos));
}

#[test]
fn gamepad_edge_cursor_does_not_pan_without_a_viewport() {
    let mut app = edge_pan_app();
    app.update();
    let baseline = camera_xy(&mut app);

    send_cursor(&mut app, WINDOW * 0.5);
    app.update();
    assert_eq!(
        camera_xy(&mut app),
        baseline,
        "a centred gamepad cursor (no edge band) must NOT move the camera",
    );

    send_cursor(&mut app, Vec2::new(WINDOW.x * 0.5, 5.0));
    app.update();
    assert_eq!(
        camera_xy(&mut app),
        baseline,
        "with no map-viewport rect (headless), an edge-band gamepad cursor must NOT pan",
    );
}

#[test]
fn no_message_means_no_pan() {
    let mut app = edge_pan_app();
    app.update();
    let baseline = camera_xy(&mut app);

    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        camera_xy(&mut app),
        baseline,
        "with no GamepadCursorMoved message the camera must not pan",
    );
}
