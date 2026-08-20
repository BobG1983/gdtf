use super::support::*;
use crate::acts::movement::{ReactionShotFired, WalkInProgress};

/// Frames a walk gets to run itself out before a case gives up on it.
const WALK_BUDGET: u8 = 16;

// Run until the walk is over, collecting both movement buffers as they fill.
fn walk_it_out(app: &mut App) -> (Vec<MovementOccurred>, Vec<MoveCompleted>) {
    let mut steps: Vec<MovementOccurred> = Vec::new();
    let mut arrivals: Vec<MoveCompleted> = Vec::new();
    for _ in 0..WALK_BUDGET {
        app.update();
        steps.extend(drain_movements(app));
        arrivals.extend(drain_completed_moves(app));
    }
    (steps, arrivals)
}

#[test]
fn a_multi_cell_walk_announces_one_completed_move_however_many_cells_it_stepped() {
    let mut app = headless_app();
    let walker = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(14, 10), Level::new(0));

    app.world_mut()
        .write_message(MoveRequested::new(walker, dest));
    let (steps, arrivals) = walk_it_out(&mut app);

    assert!(
        steps.len() >= 4,
        "the fixture walks four cells, so the walk announces at least four steps — got \
         {step_count} steps against {move_count} completed moves",
        step_count = steps.len(),
        move_count = arrivals.len(),
    );
    assert_eq!(
        arrivals.len(),
        1,
        "a walk is one completed move however many cells it steps — got {move_count} \
         completed moves against {step_count} steps",
        move_count = arrivals.len(),
        step_count = steps.len(),
    );
    let Some(arrival) = arrivals.first() else {
        unreachable!("the assertion above requires exactly one completed move");
    };
    assert_eq!(arrival.mover, walker, "the message names the walker");
    assert_eq!(
        arrival.at, dest,
        "the message carries the cell and level the walk ended on",
    );
    assert!(
        app.world().get::<WalkInProgress>(walker).is_none(),
        "the walk is over by the time the case reads its messages",
    );
}

#[test]
fn a_walk_stopped_before_its_first_step_announces_nothing() {
    let mut app = headless_app();
    let walker = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(14, 10), Level::new(0));

    app.world_mut()
        .write_message(MoveRequested::new(walker, dest));
    app.world_mut()
        .write_message(ReactionShotFired::new(walker));
    let (steps, arrivals) = walk_it_out(&mut app);

    assert!(
        steps.is_empty(),
        "reaction fire landed before the first step, so no cell was ever popped: {steps:?}",
    );
    assert!(
        arrivals.is_empty(),
        "a walk that never left its start cell completed no move: {arrivals:?}",
    );
    assert_eq!(
        app.world().get::<Position>(walker).copied(),
        Some(Position::new(CellLevel::new(
            Cell::new(10, 10),
            Level::new(0)
        ))),
        "the interrupted mover is still standing where it started",
    );
}
