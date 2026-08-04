use bevy::prelude::Entity;
use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::{harness::*, support::*};

fn spend_totals(app: &bevy::app::App, reactors: [Entity; 4]) -> (u32, usize) {
    let used: u32 = reactors
        .into_iter()
        .filter_map(|entity| used_of(app, entity))
        .sum();
    let shots: usize = reactors
        .into_iter()
        .map(|entity| shots_by(app, entity))
        .sum();
    (used, shots)
}


#[test]
fn two_movers_one_tick_spend_only_what_actually_fires() {
    let mut app = battle_app(forced_reaction_tuning(2));
    with_shot_log(&mut app);

    let watcher_cell = ground(5, 5);
    let north_start = ground(7, 4);
    let south_start = ground(7, 6);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watcher_cell, PLAYER, Direction::East),
            tough_mover(north_start, ENEMY, Direction::East),
            tough_mover(south_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(watcher_entity), Some(north), Some(south)) = (
        ganger_at(&mut app, watcher_cell),
        ganger_at(&mut app, north_start),
        ganger_at(&mut app, south_start),
    ) else {
        unreachable!("setup spawns the watcher and both movers at their fixture cells");
    };

    let cost = single_fire_cost(&mut app, watcher_entity);
    assert!(
        cost >= 1,
        "fixture precondition: the single-shot charge is non-zero ({cost}), else \
         one-shot affordability is unconstructible",
    );
    set_tu(&mut app, watcher_entity, cost);

    app.world_mut()
        .write_message(MoveRequested::new(north, ground(11, 4)));
    app.world_mut()
        .write_message(MoveRequested::new(south, ground(11, 6)));
    step(&mut app, 16);

    let shots = shots_by(&app, watcher_entity);
    assert_eq!(
        shots, 1,
        "fixture precondition: exactly ONE interrupt shot dispatches (the pool affords \
         one single-mode charge)",
    );
    assert_eq!(
        used_of(&app, watcher_entity),
        u32::try_from(shots).ok(),
        "ReactionsUsed increments 1:1 with actually-dispatched reaction shots \
         (a second same-tick interrupt offered on a stale TU snapshot must not consume \
         the cap when the dispatcher rejects it)",
    );
}


#[test]
fn mixed_ineligible_reactors_leave_the_counter_and_tu_untouched() {
    let mut app = battle_app(forced_reaction_tuning(1));
    with_shot_log(&mut app);

    let eligible_cell = ground(4, 4);
    let unaffordable_cell = ground(4, 1);
    let suppressed_cell = ground(4, 7);
    let arc_reject_cell = ground(4, 10);
    let mover_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(eligible_cell, PLAYER, Direction::East),
            watcher(unaffordable_cell, PLAYER, Direction::East),
            watcher(suppressed_cell, PLAYER, Direction::East),
            watcher(arc_reject_cell, PLAYER, Direction::West),
            tough_mover(mover_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(eligible), Some(unaffordable), Some(suppressed_reactor), Some(arc_rejected)) = (
        ganger_at(&mut app, eligible_cell),
        ganger_at(&mut app, unaffordable_cell),
        ganger_at(&mut app, suppressed_cell),
        ganger_at(&mut app, arc_reject_cell),
    ) else {
        unreachable!("setup spawns all four reactors at their fixture cells");
    };
    let Some(mover) = ganger_at(&mut app, mover_start) else {
        unreachable!("setup spawns the mover at its fixture cell");
    };

    {
        let tuning = app
            .world()
            .resource::<gdtf_battle_sim::tuning::CombatTuning>();
        assert!(
            *tuning.turn_tu > 0,
            "fixture precondition: turning costs TU, else out-of-arc is never rejected",
        );
    }
    let cost = single_fire_cost(&mut app, eligible);
    assert!(
        cost >= 2,
        "fixture precondition: the single-shot charge ({cost}) leaves room for a \
         non-zero-but-unaffordable pool",
    );
    set_tu(&mut app, unaffordable, cost - 1);
    set_tu(&mut app, arc_rejected, cost);
    suppress(&mut app, suppressed_reactor, mover_start);

    let tu_before = |app: &bevy::app::App| {
        (
            tu_of(app, unaffordable),
            tu_of(app, suppressed_reactor),
            tu_of(app, arc_rejected),
        )
    };
    let ineligible_tu = tu_before(&app);

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    step(&mut app, 16);

    assert_eq!(
        shots_by(&app, eligible),
        1,
        "the eligible watcher dispatched exactly one interrupt (cap == 1)",
    );
    assert_eq!(
        used_of(&app, eligible),
        Some(1),
        "the eligible watcher's counter moved exactly once — 1:1 with its shot",
    );

    for (name, entity) in [
        ("unaffordable", unaffordable),
        ("suppressed", suppressed_reactor),
        ("arc-rejected", arc_rejected),
    ] {
        assert_eq!(
            shots_by(&app, entity),
            0,
            "the {name} reactor dispatched no reaction shot",
        );
        assert_eq!(
            used_of(&app, entity),
            Some(0),
            "the {name} reactor's ReactionsUsed counter is untouched",
        );
    }
    assert_eq!(
        tu_before(&app),
        ineligible_tu,
        "no ineligible reactor spent any TU (unaffordable / suppressed / arc-rejected)",
    );

    let (total_used, total_shots) = spend_totals(
        &app,
        [eligible, unaffordable, suppressed_reactor, arc_rejected],
    );
    assert_eq!(
        u32::try_from(total_shots).ok(),
        Some(total_used),
        "ReactionsUsed increments correspond 1:1 with dispatched reaction \
         shots across the mixed tick",
    );
}
