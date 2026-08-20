use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq},
    acts::{FireRequested, MoveRequested},
    ganger::Direction,
    test_support::SituationBuilder,
};

use super::harness::*;

#[test]
fn two_same_tick_interrupts_append_two_ordered_reaction_entries() {
    let mut app = battle_app(forced_reaction_tuning(8));

    let north_watch = ground(5, 4);
    let south_watch = ground(5, 6);
    let mover_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(north_watch, PLAYER, Direction::East),
            watcher(south_watch, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(north), Some(south), Some(mover)) = (
        ganger_at(&mut app, north_watch),
        ganger_at(&mut app, south_watch),
        ganger_at(&mut app, mover_start),
    ) else {
        unreachable!("setup spawns both watchers and the mover at their fixture cells");
    };

    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    for _ in 0..6 {
        app.update();
    }

    let reactions: Vec<_> = logged_of(&app, "Fired")
        .into_iter()
        .filter(|fact| fact.provenance.is_reaction())
        .collect();

    assert!(
        reactions.len() >= 2,
        "two reactors interrupting one step must produce at least two REACTION-provenance \
         fire entries — got {} (all fire entries: {:?})",
        reactions.len(),
        logged_of(&app, "Fired"),
    );

    let actors: Vec<_> = reactions.iter().map(|fact| fact.actor).collect();
    assert!(
        actors.contains(&north) && actors.contains(&south),
        "each reactor must own its OWN reaction entry — north {north:?} and south {south:?} \
         must both appear among {actors:?}",
    );

    for fact in &reactions {
        assert_eq!(
            fact.provenance,
            ActProvenance::Reaction { interrupted: mover },
            "a reaction entry must name the interrupted mover, not the reactor or nothing",
        );
    }

    let seqs: Vec<ActSeq> = reactions.iter().map(|fact| fact.seq).collect();
    let mut sorted = seqs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        seqs, sorted,
        "reaction entries must be strictly increasing in sequence — two interrupts are two \
         events in an order, never one indistinguishable blur",
    );
}

#[test]
fn one_reactor_firing_twice_in_a_tick_pairs_rounds_to_the_right_declaration() {
    let mut app = battle_app(forced_reaction_tuning(8));

    let shooter_cell = ground(5, 5);
    let target_cell = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(shooter_cell, PLAYER, Direction::East),
            tough_mover(target_cell, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(shooter), Some(target)) = (
        ganger_at(&mut app, shooter_cell),
        ganger_at(&mut app, target_cell),
    ) else {
        unreachable!("setup spawns the shooter and the target at their fixture cells");
    };
    set_tu_max(&mut app, shooter, 40);
    set_tu(&mut app, shooter, u8::MAX);
    load_magazine(&mut app, shooter, 12);

    let mode = single_mode_of(&mut app, shooter);
    let (cell, level) = target_cell.split();
    app.world_mut()
        .write_message(FireRequested::new(shooter, mode, cell, level));
    app.world_mut()
        .write_message(FireRequested::new(shooter, mode, cell, level));
    app.update();

    let Some(log) = app.world().get_resource::<ActLog>() else {
        unreachable!("a live battle carries an act log");
    };
    let entries: Vec<_> = log.since(ActSeq::START).collect();

    let mut declarations = 0_usize;
    for (index, entry) in entries.iter().enumerate() {
        if entry.actor() != shooter {
            continue;
        }
        let ActDeed::Fired { rounds, .. } = entry.deed() else {
            continue;
        };
        declarations += 1;
        let paired = entries
            .iter()
            .skip(index + 1)
            .take_while(|next| {
                next.actor() == shooter && matches!(next.deed(), ActDeed::RoundResolved { .. })
            })
            .count();
        assert!(
            **rounds > 0,
            "a declaration that proceeded emitted at least one round",
        );
        assert_eq!(
            u32::try_from(paired).unwrap_or(u32::MAX),
            **rounds,
            "the declaration at seq {:?} announced {} rounds, but {paired} round entries \
             follow it — a greedy leading-run pairing hands the FIRST declaration of a tick \
             every round and leaves the second with none",
            entry.seq(),
            **rounds,
        );
    }

    assert_eq!(
        declarations, 2,
        "the fixture must produce exactly two same-tick declarations from ONE shooter, else \
         the pairing hazard is not exercised at all",
    );
    assert!(
        !logged_of(&app, "RoundResolved").is_empty(),
        "the fixture must actually resolve rounds; target {target:?}",
    );
}
