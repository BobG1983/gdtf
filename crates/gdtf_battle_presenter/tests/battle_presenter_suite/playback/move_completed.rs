use std::time::Duration;

use gdtf_battle_sim::{
    act_log::{ActDeed, ActProvenance, PositionFacts},
    acts::{MoveCompleted, MovementOccurred},
    ganger::{Direction, Position},
};

use super::harness::*;

#[test]
fn a_recorded_completed_move_is_played_with_its_mover_and_final_cell() {
    let mut app = playback_app();
    let walker = spawn_ganger(&mut app, ground(1, 1), Direction::North);
    seed(&mut app);

    let destination = ground(4, 7);
    append(
        &mut app,
        walker,
        ActProvenance::Commanded,
        ActDeed::MovedTo {
            position: PositionFacts::new(Position::new(destination)),
        },
    );
    step(&mut app, Duration::ZERO);

    let completions = played::<MoveCompleted>(&mut app);
    assert_eq!(
        completions.len(),
        1,
        "showing a MovedTo deed must emit exactly one Played<MoveCompleted> — it is the only \
         thing that puts a movement line on the combat log panel; got {completions:?}",
    );
    let Some(completed) = completions.first() else {
        return;
    };
    assert_eq!(
        completed.mover, walker,
        "the played move names the mover the entry recorded; got {completed:?}",
    );
    assert_eq!(
        completed.at, destination,
        "the played move carries the cell the entry recorded — the panel gates its movement \
         line on this cell, so a fact built from any other cell is kept or dropped against a \
         cell nothing happened on; got {completed:?}",
    );
}

#[test]
fn a_recorded_step_plays_a_step_and_no_completed_move() {
    let mut app = playback_app();
    let walker = spawn_ganger(&mut app, ground(2, 2), Direction::East);
    seed(&mut app);

    let from = ground(2, 2);
    let to = ground(3, 2);
    append(
        &mut app,
        walker,
        ActProvenance::Commanded,
        ActDeed::Stepped {
            from:     from.cell(),
            to:       to.cell(),
            position: PositionFacts::new(Position::new(to)),
        },
    );
    step(&mut app, Duration::ZERO);

    assert_eq!(
        played::<MovementOccurred>(&mut app).len(),
        1,
        "a Stepped deed still plays its step",
    );
    let completions = played::<MoveCompleted>(&mut app);
    assert!(
        completions.is_empty(),
        "only a MovedTo deed completes a move — a step must play none, else every cell of a \
         walk would put its own line on the panel; got {completions:?}",
    );
}
