//! One-shot frame-on-units centroid + latch (AC3).

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    ecs::schedule::SystemCondition,
    math::Vec2,
    prelude::{Camera2d, IntoScheduleConfigs, Transform, With, resource_exists},
};
use gdtf_battle_presenter::{WorldCamera, camera_focus, cell_to_world, frame_camera_on_units};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Faction, Level, PlayerFaction, Position};

use super::harness::*;

/// The known cells the two player gangers occupy in AC3 (ground storey).
const PLAYER_CELLS: [(i32, i32); 2] = [(10, 12), (20, 24)];
/// The enemy ganger's cell (off in a corner, to prove it does not move the centroid).
const ENEMY_CELL: (i32, i32) = (55, 3);

/// Builds a `Position` on the ground storey from a `(x, y)` cell.
fn ground_position(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

/// The world-space centroid the AC3 framing must snap the camera to: the mean of the two
/// player gangers' `cell_to_world` ground positions (the SAME projection the systems use).
fn expected_player_focus() -> Vec2 {
    let centers = PLAYER_CELLS.into_iter().map(|(x, y)| {
        let world = cell_to_world(Cell::new(x, y), Level::new(0));
        Vec2::new(world.x, world.y)
    });
    camera_focus(centers).unwrap_or(Vec2::ZERO)
}

// ---------------------------------------------------------------------------------
// AC3 — frame-on-units centres on the player centroid ONCE; the latch holds.
// ---------------------------------------------------------------------------------

/// AC3 — one update centres the `WorldCamera` on the player gangers' world centroid, and
/// further updates do NOT re-centre (the one-shot latch holds), so the framing won't fight
/// the later pan nav.
#[test]
fn frame_on_units_centres_once_and_latches() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        frame_camera_on_units
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>)),
    );

    // Battle-scoped witnesses the system gates on.
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));

    // The camera starts at the origin (the static spawn never moves it).
    app.world_mut()
        .spawn((Camera2d, WorldCamera, Transform::default()));

    // Two player gangers at known cells + one enemy the framing must ignore.
    for (x, y) in PLAYER_CELLS {
        app.world_mut()
            .spawn((Faction::new(PLAYER_GANG), ground_position(x, y)));
    }
    app.world_mut().spawn((
        Faction::new(ENEMY_GANG),
        ground_position(ENEMY_CELL.0, ENEMY_CELL.1),
    ));

    // One update frames the camera on the player centroid.
    app.update();
    let focus = expected_player_focus();
    let after_first = camera_xy(&mut app);
    assert_eq!(
        after_first, focus,
        "the camera must centre on the PLAYER gangers' world centroid (enemy ignored)",
    );

    // The framing must NOT be the origin — i.e. it actually moved (the player cells are
    // off-origin), so a passing assertion above is real centring, not a no-op.
    assert_ne!(
        after_first,
        Vec2::ZERO,
        "the player centroid is off-origin, so the camera must have moved from spawn",
    );

    // Several MORE updates: the latch holds, so a later writer (pan nav) is not fought —
    // we simulate one by nudging the camera and confirming the framing leaves it alone.
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

/// Sets the `WorldCamera` translation `xy` to `to` in a TEST BODY (simulating a later
/// camera writer such as the pan nav).
fn nudge_camera(app: &mut App, to: Vec2) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&mut Transform, With<WorldCamera>>();
    for mut transform in query.iter_mut(world) {
        transform.translation.x = to.x;
        transform.translation.y = to.y;
    }
}
