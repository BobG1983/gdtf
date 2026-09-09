//! The successor spawn runs after the deaths and before everything that reads the grids.

use bevy::ecs::schedule::NodeId;

use super::{
    schedule_support::{a_system_named, ordered_before, update_schedule},
    socket_support::{TestResult, battle_app_listening},
};

/// The systems that read a grid the successor spawn writes.
const READS_THE_GRIDS: [&str; 6] = [
    "sync_accrued_ground",
    "sync_peek_offsets",
    "recompute_visibility",
    "dispatch_move",
    "advance_walk",
    "apply_falls",
];

#[test]
fn the_successor_spawn_runs_after_the_dead_gangers_leave_the_grid() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let dead = a_system_named(update, "sync_inactive_gangers")?;
    let piece = a_system_named(update, "replace_destroyed_piece")?;
    assert!(
        ordered_before(graph, NodeId::System(dead), piece),
        "sync_inactive_gangers must be ordered before replace_destroyed_piece; both write the \
         occupancy grid and unordered the executor picks which clears the cell last",
    );
    Ok(())
}

#[test]
fn the_successor_spawn_runs_before_path_blocking_is_projected() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let piece = a_system_named(update, "replace_destroyed_piece")?;
    let blocking = a_system_named(update, "project_path_blocking")?;
    assert!(
        ordered_before(graph, NodeId::System(piece), blocking),
        "replace_destroyed_piece must be ordered before project_path_blocking, or the blocking \
         projection is rebuilt before the destroyed piece has lost its BlocksPathfinding",
    );
    Ok(())
}

#[test]
fn every_grid_reader_runs_after_the_successor_spawn() -> TestResult {
    let (mut app, _port) = battle_app_listening()?;
    app.update();
    let update = update_schedule(&app)?;
    let graph = update.graph();

    let piece = a_system_named(update, "replace_destroyed_piece")?;
    for named in READS_THE_GRIDS {
        let reader = a_system_named(update, named)?;
        assert!(
            ordered_before(graph, NodeId::System(piece), reader),
            "replace_destroyed_piece must be ordered before `{named}`; the successor spawn writes \
             both the occupancy grid and the surface grid, so a reader that only waited on one \
             half before now has to wait on the whole thing",
        );
    }
    Ok(())
}
