use bevy::{
    MinimalPlugins,
    app::{App, Update},
    ecs::schedule::SystemCondition,
    math::Vec2,
    prelude::{Camera2d, IntoScheduleConfigs, Transform, With, resource_exists},
};
use gdtf_battle_presenter::{WorldCamera, camera_focus, cell_to_world, frame_camera_on_units};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, Position},
};

use super::harness::*;

const PLAYER_CELLS: [(i32, i32); 2] = [(10, 12), (20, 24)];
const ENEMY_CELL: (i32, i32) = (55, 3);

fn ground_position(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

fn expected_player_focus() -> Vec2 {
    let centers = PLAYER_CELLS.into_iter().map(|(x, y)| {
        let world = cell_to_world(Cell::new(x, y), Level::new(0));
        Vec2::new(world.x, world.y)
    });
    camera_focus(centers).unwrap_or(Vec2::ZERO)
}


#[test]
fn frame_on_units_centres_once_and_latches() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        frame_camera_on_units
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>)),
    );

    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));

    app.world_mut()
        .spawn((Camera2d, WorldCamera, Transform::default()));

    for (x, y) in PLAYER_CELLS {
        app.world_mut()
            .spawn((Faction::new(PLAYER_GANG), ground_position(x, y)));
    }
    app.world_mut().spawn((
        Faction::new(ENEMY_GANG),
        ground_position(ENEMY_CELL.0, ENEMY_CELL.1),
    ));

    app.update();
    let focus = expected_player_focus();
    let after_first = camera_xy(&mut app);
    assert_eq!(
        after_first, focus,
        "the camera must centre on the PLAYER gangers' world centroid (enemy ignored)",
    );

    assert_ne!(
        after_first,
        Vec2::ZERO,
        "the player centroid is off-origin, so the camera must have moved from spawn",
    );

    nudge_camera(&mut app, Vec2::new(999.0, -999.0));
    for _ in 0..3 {
        app.update();
    }
    let after_nudge = camera_xy(&mut app);
    assert_eq!(
        after_nudge,
        Vec2::new(999.0, -999.0),
        "after the one-shot framing, later updates must NOT re-centre the camera (latch holds)",
    );
}

fn nudge_camera(app: &mut App, to: Vec2) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&mut Transform, With<WorldCamera>>();
    for mut transform in query.iter_mut(world) {
        transform.translation.x = to.x;
        transform.translation.y = to.y;
    }
}
