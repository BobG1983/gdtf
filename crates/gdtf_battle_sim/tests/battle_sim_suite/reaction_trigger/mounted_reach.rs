//! The controls: from its emplacement's cell each mounted reactor still sees the mover's path.

use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::{harness::*, support::*};

#[test]
fn a_loaded_mount_interrupts_from_the_seat_in_the_skipped_offer_layout() {
    let mut app = battle_app(forced_reaction_tuning(1));
    app.insert_resource(two_gun_registry());
    with_shot_log(&mut app);

    let watcher_cell = ground(5, 5);
    let emplacement_cell = ground(5, 6);
    let mover_start = ground(7, 4);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watcher_cell, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(reactor), Some(mover)) = (
        ganger_at(&mut app, watcher_cell),
        ganger_at(&mut app, mover_start),
    ) else {
        unreachable!("setup spawns the watcher and the mover at their fixture cells");
    };

    let emplacement = spawn_emplacement(&mut app, emplacement_cell);
    let _mount = man_emplacement(&mut app, reactor, emplacement);
    assert_eq!(
        pos_of(&app, reactor),
        Some(emplacement_cell),
        "the reactor is judged from the emplacement's cell now, not from where it spawned",
    );

    settle_the_enter_exchange(&mut app, &[reactor], mover);
    let baseline = shots_by(&app, reactor);

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 4)));
    step(&mut app, 16);

    assert_eq!(
        shots_by(&app, reactor) - baseline,
        1,
        "with the mount LOADED the seat at {emplacement_cell:?} sees the mover and interrupts \
         it exactly once (cap == 1) — so the empty-mount case's zero is the empty magazine, not \
         a layout that cannot see",
    );
}

#[test]
fn a_loaded_mount_interrupts_from_the_seat_in_the_mixed_tick_layout() {
    let mut app = battle_app(forced_reaction_tuning(1));
    app.insert_resource(two_gun_registry());
    with_shot_log(&mut app);

    let mounted_cell = ground(4, 4);
    let eligible_cell = ground(4, 7);
    let emplacement_cell = ground(4, 3);
    let mover_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(mounted_cell, PLAYER, Direction::East),
            watcher(eligible_cell, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(mounted_reactor), Some(eligible), Some(mover)) = (
        ganger_at(&mut app, mounted_cell),
        ganger_at(&mut app, eligible_cell),
        ganger_at(&mut app, mover_start),
    ) else {
        unreachable!("setup spawns both reactors and the mover at their fixture cells");
    };

    let emplacement = spawn_emplacement(&mut app, emplacement_cell);
    let _mount = man_emplacement(&mut app, mounted_reactor, emplacement);
    assert_eq!(
        pos_of(&app, mounted_reactor),
        Some(emplacement_cell),
        "the mounted reactor is judged from the emplacement's cell now, not from where it \
         spawned",
    );

    settle_the_enter_exchange(&mut app, &[mounted_reactor, eligible], mover);
    let baseline = shots_by(&app, mounted_reactor);

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    step(&mut app, 16);

    assert_eq!(
        shots_by(&app, mounted_reactor) - baseline,
        1,
        "with the mount LOADED the seat at {emplacement_cell:?} sees the mover and interrupts \
         it exactly once (cap == 1) — so the mixed-tick case's zero is the empty magazine, not \
         a layout that cannot see",
    );
}
