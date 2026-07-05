//! When an overwatch interrupt fires (and does not): in-LOS acting, outside-LOS
//! immunity, enemy-turn faction symmetry, and interrupt-sequence determinism
//! (AC1 / AC2 / AC3 / AC5 / AC7).

use gdtf_battle_sim::{
    ReactionShotFired,
    acts::{EndTurnRequested, MoveRequested},
    ganger::Direction,
    test_support::SituationBuilder,
};

use super::harness::*;

// === AC1 + AC2 + AC7 — a PLAYER moving in an ENEMY watcher's LOS is interrupted (the
// watcher's TU debits, a normal ShotFired fires at the player, the walk halts). ===

#[test]
fn ac1_ac2_ac7_acting_player_is_interrupted_by_an_enemy_watcher() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    // The enemy watcher sits west, facing East down the player's walk lane; the player
    // stands a few cells east of it and walks further east — every step is in the watcher's
    // LOS + arc + range. (Player turn is active at setup, so the player MoveRequested
    // dispatches immediately.)
    let watcher_cell = ground(4, 5);
    let player_start = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            mover(player_start, PLAYER, Direction::East),
            watcher(watcher_cell, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(enemy_tu_before) = tu_of(&app, enemy) else {
        unreachable!("the enemy has a Tu pool");
    };
    let Some(player_start_now) = pos_of(&app, player) else {
        unreachable!("the player has a Position");
    };

    // The player walks a multi-cell route east (in the watcher's LOS the whole way).
    let dest = ground(10, 5);
    app.world_mut()
        .write_message(MoveRequested::new(player, dest));
    run_until_walk_ends(&mut app, player);
    // A couple more ticks so the one-tick-cadence interrupt (read the step's Changed<Position>
    // next tick → fire) fully resolves and is recorded.
    step(&mut app, 3);

    // AC2: the walk HALTED short of the destination (the live ReactionShotFired removed the
    // WalkInProgress).
    let Some(player_final) = pos_of(&app, player) else {
        unreachable!("the player persists");
    };
    assert!(
        !is_walking(&app, player),
        "AC2: the interrupt removes the player's WalkInProgress (the walk halted)",
    );
    assert_ne!(
        player_final, dest,
        "AC2: the interrupt halts the walk SHORT of the destination",
    );
    assert_ne!(
        player_final, player_start_now,
        "the player took at least one step before the interrupt fired",
    );

    // AC1: the enemy watcher's TU was DEBITED (the interrupt shot spent its fire TU through
    // the normal fire pipeline).
    let Some(enemy_tu_after) = tu_of(&app, enemy) else {
        unreachable!("the enemy persists");
    };
    assert!(
        enemy_tu_after < enemy_tu_before,
        "AC1: the watcher's TU is debited by the interrupt shot ({enemy_tu_after} < \
         {enemy_tu_before})",
    );

    // AC1 (positive, pin-discriminating): a REAL ShotFired came FROM the enemy watcher —
    // the interrupt actually fired (this fails if the trigger is unwired).
    assert!(
        shots_by(&app, enemy) >= 1,
        "AC1: the enemy watcher fired at least one interrupt round (ShotFired from the \
         watcher) — the trigger is LIVE",
    );

    // AC7: every round the watcher fired is a NORMAL-pipeline shot — it carries a HitReport
    // (a bespoke reaction path would not resolve through dispatch_fire → fire()).
    assert!(
        all_shots_have_reports(&app, enemy),
        "AC7: the reaction shot resolves through the NORMAL fire pipeline (every ShotFired \
         carries a HitReport — no reaction damage modifier / no bespoke path)",
    );

    // The reactor consumed its per-turn cap at least once (the count incremented — C4).
    assert!(
        used_of(&app, enemy).is_some_and(|u| u >= 1),
        "the watcher's ReactionsUsed incremented when it interrupted",
    );
}

// === AC3 — the LOS gate. An actor acting OUTSIDE all opposing watchers' LOS produces NO
// interrupt; the SAME act with the watcher in LOS DOES. ===

#[test]
fn ac3_no_interrupt_when_the_actor_acts_outside_los() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    // The watcher is parked FAR (well beyond TEST_VIEW_RANGE) from the player's walk lane,
    // so the moving player is never in range — NO interrupt despite forced p == 1.0.
    let watcher_cell = ground(40, 40);
    let player_start = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            mover(player_start, PLAYER, Direction::East),
            watcher(watcher_cell, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(enemy_tu_before) = tu_of(&app, enemy) else {
        unreachable!("the enemy has a Tu pool");
    };

    let dest = ground(10, 5);
    app.world_mut()
        .write_message(MoveRequested::new(player, dest));
    run_until_walk_ends(&mut app, player);
    step(&mut app, 3);

    // AC3 (negative): NO interrupt — the watcher fired nothing, spent no TU, and the player
    // walked freely to its destination.
    assert_eq!(
        shots_by(&app, enemy),
        0,
        "AC3: an out-of-range watcher fires NO interrupt (LOS/range gate held)",
    );
    assert_eq!(
        tu_of(&app, enemy),
        Some(enemy_tu_before),
        "AC3: the out-of-range watcher spent no TU",
    );
    assert_eq!(
        used_of(&app, enemy),
        Some(0),
        "AC3: the out-of-range watcher's cap counter never incremented",
    );
    assert_eq!(
        pos_of(&app, player),
        Some(dest),
        "AC3: with no interrupt the player walks freely to its destination",
    );
}

// === AC5 — faction symmetry. The enemy-watches-player case is covered by AC1; here we drive
// the MIRROR through the brain: a PLAYER watcher interrupts an acting ENEMY during the enemy
// turn (the brain emits the enemy's move, which steps in the player watcher's LOS). ===

#[test]
fn ac5_a_player_watcher_interrupts_an_acting_enemy_on_the_enemy_turn() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    // A player watcher facing East at (5,5), view range 6. The enemy starts FAR east at
    // (15,5) — Chebyshev 10 > 6, so it is OUT of the watcher's range AND out of its own
    // engage range to the watcher at spawn. So on the enemy turn the brain ADVANCES the
    // enemy toward its nearest opposing ganger (the watcher), walking WEST down row 5. The
    // STEP that carries the enemy to within view range (Chebyshev 6, i.e. cell (11,5)) is the
    // act-in-LOS the player watcher interrupts — driven THROUGH the brain/move path, not a
    // synthetic emit. (The enemy starting out of engage range is what makes it MOVE rather
    // than open fire first — the move surface, the mirror of AC1's player move.)
    let player_watcher = ground(5, 5);
    let enemy_at = ground(15, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            // The player watcher (faction 0) — high Reactions, facing the enemy's lane.
            watcher(player_watcher, PLAYER, Direction::East),
            // The acting enemy (faction 1) — out of range at spawn, advanced by the brain.
            mover(enemy_at, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(player_watcher_entity) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns the player watcher");
    };
    let Some(enemy) = ganger_of(&mut app, ENEMY) else {
        unreachable!("setup spawns the enemy");
    };
    let Some(watcher_tu_before) = tu_of(&app, player_watcher_entity) else {
        unreachable!("the player watcher has a Tu pool");
    };

    // End the PLAYER turn → control passes to the enemy; the brain drives the enemy across
    // the following ticks (it advances toward the distant player goal, stepping through the
    // player watcher's LOS). Give it a generous budget to act + be interrupted + the
    // one-tick cadence to resolve.
    app.world_mut().write_message(EndTurnRequested);
    // Drive the enemy turn: the brain advances the enemy west toward the watcher; the step
    // into the watcher's view range is interrupted (the one-tick cadence resolves within a
    // few ticks of that step). A generous budget so the brain advances + the interrupt fires.
    step(&mut app, 16);

    // AC5: a player watcher interrupted the acting enemy during the ENEMY turn — a real
    // ShotFired from the player watcher, and its TU debited (the persistent ShotLog + TU
    // capture the interrupt regardless of the later turn-boundary cap reset). The enemy was
    // BRAIN-DRIVEN (its move, not a synthetic emit) — proven by its advance into range.
    assert!(
        shots_by(&app, player_watcher_entity) >= 1,
        "AC5: the PLAYER watcher fired an interrupt at the BRAIN-driven acting enemy during \
         the enemy turn (faction symmetry — the mirror of AC1)",
    );
    assert!(
        tu_of(&app, player_watcher_entity).is_some_and(|tu| tu < watcher_tu_before),
        "AC5: the player watcher's TU is debited by its interrupt of the enemy",
    );
    // AC7 mirror: the player watcher's interrupt is also a NORMAL-pipeline shot.
    assert!(
        all_shots_have_reports(&app, player_watcher_entity),
        "AC5/AC7: the player watcher's interrupt resolves through the normal fire pipeline",
    );
    // The enemy was the brain-driven actor whose advance the watcher interrupted (it moved
    // off its spawn cell — never a synthetic emit).
    assert!(
        pos_of(&app, enemy).is_some_and(|p| p != enemy_at),
        "AC5: the enemy ACTUALLY moved (the brain drove it) — the interrupt saw a real step",
    );
}

// === A defensive end-to-end determinism check: the same seed + tuning reproduces the same
// interrupt outcome (count of watcher shots), proving the seeded ReactionRng draw is
// replay-stable through the live path (C3). ===

#[test]
fn the_interrupt_sequence_is_deterministic_across_identical_runs() {
    let run_once = || {
        let mut app = battle_app(forced_reaction_tuning(8));
        with_shot_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                mover(ground(6, 5), PLAYER, Direction::East),
                watcher(ground(4, 5), ENEMY, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("setup spawns one player and one enemy");
        };
        app.world_mut()
            .write_message(MoveRequested::new(player, ground(10, 5)));
        run_until_walk_ends(&mut app, player);
        step(&mut app, 3);
        (shots_by(&app, enemy), pos_of(&app, player))
    };

    let first = run_once();
    let second = run_once();
    assert_eq!(
        first, second,
        "the same seed + tuning reproduces the same interrupt outcome (replay-stable \
         ReactionRng through the live path)",
    );
    // And it's a non-trivial outcome (the interrupt actually fired — otherwise determinism is
    // vacuous).
    assert!(
        first.0 >= 1,
        "precondition: the deterministic run actually interrupts (forced p == 1.0)",
    );
    // Touch the synthetic-vs-live distinction marker so the import is exercised: the live
    // ReactionShotFired producer (this module) is what halts the walk — the test never emits
    // it synthetically.
    let _ = ReactionShotFired::new;
}
