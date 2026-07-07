//! GTW-646 — reaction-cap SPEND INTEGRITY: `ReactionsUsed` increments correspond 1:1
//! with actually-dispatched reaction shots.
//!
//! The filed mechanism: the trigger's eligibility gates read the reactor's TU from a
//! per-pass snapshot while the spend settles later in `dispatch_fire` — so with a cap
//! above one and TWO actors acting in ONE tick, the second interrupt can look
//! affordable on the stale snapshot, get rejected by the dispatcher, yet the cap
//! counter was already incremented: a reaction spent on a non-shot.

use bevy::prelude::Entity;
use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::{harness::*, support::*};

/// The `(total ReactionsUsed, total dispatched rounds)` across `reactors` — the two
/// sides of the C3 global 1:1 invariant.
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

// === C1 — the filed stale-snapshot geometry: cap == 2, TWO movers stepping in one
// tick, reactor TU affording exactly ONE interrupt shot. The cap spend must track the
// DISPATCHED shot count (1), never the offer count (2). ===

#[test]
fn two_movers_one_tick_spend_only_what_actually_fires() {
    // cap == 2 (the >1 cap the shipped `floor(cap_base + cap_per_reactions×Reactions)`
    // formula reaches at Reactions >= 2), forced p == 1.0 so both opposed checks would
    // succeed — the ONLY thing separating one interrupt from two is TU affordability.
    let mut app = battle_app(forced_reaction_tuning(2));
    with_shot_log(&mut app);

    // The PLAYER watcher at (5,5) facing East; TWO tough enemy movers NE and SE of it,
    // both inside its 120° East arc + LOS + view range at spawn (so neither walk
    // reveal-halts and both movers' first steps land the SAME tick).
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

    // Pin the watcher's pool to EXACTLY one single-shot charge (both movers are
    // in-arc, so no turn cost enters): the first interrupt is affordable, a second is
    // affordable ONLY on a stale snapshot that missed the first one's pending spend.
    let cost = single_fire_cost(&mut app, watcher_entity);
    assert!(
        cost >= 1,
        "fixture precondition: the single-shot charge is non-zero ({cost}), else \
         one-shot affordability is unconstructible",
    );
    set_tu(&mut app, watcher_entity, cost);

    // BOTH movers step the same ticks: two MoveRequested written together, walks
    // advance in lockstep — two act-in-LOS actors in one trigger pass.
    app.world_mut()
        .write_message(MoveRequested::new(north, ground(11, 4)));
    app.world_mut()
        .write_message(MoveRequested::new(south, ground(11, 6)));
    step(&mut app, 16);

    // The watcher could DISPATCH exactly one interrupt (TU == one charge).
    let shots = shots_by(&app, watcher_entity);
    assert_eq!(
        shots, 1,
        "fixture precondition: exactly ONE interrupt shot dispatches (the pool affords \
         one single-mode charge)",
    );
    // THE GTW-646 PIN — the spend follows the SHOT, not the offer: the per-turn cap
    // counter equals the number of reaction shots that actually dispatched. A counter
    // above the shot count is a reaction spent on a non-shot (the filed defect).
    assert_eq!(
        used_of(&app, watcher_entity),
        u32::try_from(shots).ok(),
        "GTW-646: ReactionsUsed increments 1:1 with actually-dispatched reaction shots \
         (a second same-tick interrupt offered on a stale TU snapshot must not consume \
         the cap when the dispatcher rejects it)",
    );
}

// === C3 — the mixed-tick invariant: across one act with eligible, suppressed,
// unaffordable, and arc-rejected reactors, ONLY the eligible reactor's counter moves —
// and it moves exactly as many times as it actually fired. ===

#[test]
fn mixed_ineligible_reactors_leave_the_counter_and_tu_untouched() {
    // cap == 1, forced p == 1.0: the eligible watcher fires exactly once; every
    // ineligible reactor must be skipped in its gates with NO counter/TU movement.
    let mut app = battle_app(forced_reaction_tuning(1));
    with_shot_log(&mut app);

    // Four PLAYER reactors in column x=4 (rays to the mover's path never re-cross the
    // column, so no reactor occludes another): the ELIGIBLE watcher, an UNAFFORDABLE
    // one (TU below one charge), a SUPPRESSED one, and an ARC-REJECTED one (facing
    // West with a pool that affords the shot but not the turn-into-arc). One tough
    // ENEMY mover walks east through all four sightlines.
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

    // The turn-into-arc charge must be real for the arc-reject geometry to bite.
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
    // UNAFFORDABLE: a live pool (passes the Tu > 0 eligibility gate) below one charge.
    set_tu(&mut app, unaffordable, cost - 1);
    // ARC-REJECTED: affords the shot alone, but NOT the West→(mover) turn + shot.
    set_tu(&mut app, arc_rejected, cost);
    // SUPPRESSED: full pool, clear LOS — only the marker keeps its head down.
    suppress(&mut app, suppressed_reactor, mover_start);

    let tu_before = |app: &bevy::app::App| {
        (
            tu_of(app, unaffordable),
            tu_of(app, suppressed_reactor),
            tu_of(app, arc_rejected),
        )
    };
    let ineligible_tu = tu_before(&app);

    // The act: the mover steps east in every reactor's sightline.
    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    step(&mut app, 16);

    // The eligible watcher fired exactly once and spent exactly one cap point.
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

    // Every INELIGIBLE reactor: no shot, no cap spend, no TU spend.
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

    // The global 1:1 invariant across the mixed tick: total PLAYER cap spend equals
    // total PLAYER reaction rounds dispatched.
    let (total_used, total_shots) = spend_totals(
        &app,
        [eligible, unaffordable, suppressed_reactor, arc_rejected],
    );
    assert_eq!(
        u32::try_from(total_shots).ok(),
        Some(total_used),
        "GTW-646 C3: ReactionsUsed increments correspond 1:1 with dispatched reaction \
         shots across the mixed tick",
    );
}
