use bevy::app::App;
use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::{harness::*, support::*};

fn run_mover_past_watcher(suppress_watcher: bool) -> (usize, Option<u32>, Option<u8>, Option<u8>) {
    let mut app: App = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    let watcher_cell = ground(5, 5);
    let mover_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watcher_cell, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(watcher_entity), Some(mover)) = (
        ganger_at(&mut app, watcher_cell),
        ganger_at(&mut app, mover_start),
    ) else {
        unreachable!("setup spawns the watcher and the mover at their fixture cells");
    };

    if suppress_watcher {
        suppress(&mut app, watcher_entity, mover_start);
        step(&mut app, 1);
    }
    let tu_before = tu_of(&app, watcher_entity);

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    run_until_walk_ends(&mut app, mover);
    step(&mut app, 3);

    (
        shots_by(&app, watcher_entity),
        used_of(&app, watcher_entity),
        tu_before,
        tu_of(&app, watcher_entity),
    )
}

#[test]
fn a_suppressed_reactor_neither_fires_nor_spends_cap_nor_tu() {
    let (control_shots, control_used, ..) = run_mover_past_watcher(false);
    assert!(
        control_shots >= 1,
        "control precondition: the unsuppressed watcher interrupts the mover \
         ({control_shots} shots)",
    );
    assert!(
        control_used.is_some_and(|used| used >= 1),
        "control precondition: the unsuppressed watcher's cap counter moved \
         ({control_used:?})",
    );

    let (shots, used, tu_before, tu_after) = run_mover_past_watcher(true);
    assert_eq!(
        shots, 0,
        "a Suppressed reactor with TU and LOS fires NO reaction shot at a triggering \
         mover (USER RULING)",
    );
    assert_eq!(
        used,
        Some(0),
        "a Suppressed reactor spends NO ReactionsUsed — the cap is untouched",
    );
    assert_eq!(
        tu_after, tu_before,
        "a Suppressed reactor spends NO TU — its pool is untouched across the act",
    );
}
