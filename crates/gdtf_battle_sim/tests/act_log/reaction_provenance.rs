//! T1 / T2 — the acceptance criterion as a mechanical assertion, and the exact
//! declaration→rounds pairing that makes it hold when one reactor fires twice.

use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq},
    acts::{FireRequested, MoveRequested},
    ganger::Direction,
    test_support::SituationBuilder,
};

use super::harness::*;

/// **T1 — THE ACCEPTANCE CRITERION.** Two reactors interrupting ONE mover's step in ONE
/// tick must produce TWO separately-identifiable reaction entries: distinct actors, both
/// naming the mover as the ganger they interrupted, in strictly increasing sequence order.
///
/// This is the exact situation the ticket was filed about — "both enemies reaction-fired on
/// one player step and it was impossible to tell who was shooting". Before the act log the
/// sim had no way to say it at all: an interrupt writes a plain `FireRequested`
/// indistinguishable from any other shot, and the only reaction-flavoured signal names the
/// INTERRUPTED walker rather than the reactor. So this test fails outright if the
/// provenance discriminator is dropped, and fails on ordering if the two interrupts collapse
/// into one entry.
#[test]
fn two_same_tick_interrupts_append_two_ordered_reaction_entries() {
    // Cap 8 so neither watcher is cap-limited; forced p == 1.0 so both interrupts fire.
    let mut app = battle_app(forced_reaction_tuning(8));

    // TWO player watchers flanking one enemy mover's path, both facing it, both in range.
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

    // Walk the mover past both watchers; each step is an act-in-LOS both of them see.
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

    // Both watchers are represented — the entries are separately identifiable, which is the
    // whole point.
    let actors: Vec<_> = reactions.iter().map(|fact| fact.actor).collect();
    assert!(
        actors.contains(&north) && actors.contains(&south),
        "each reactor must own its OWN reaction entry — north {north:?} and south {south:?} \
         must both appear among {actors:?}",
    );

    // Every reaction entry names the MOVER as the ganger it interrupted (the pair the sim
    // could not state before).
    for fact in &reactions {
        assert_eq!(
            fact.provenance,
            ActProvenance::Reaction { interrupted: mover },
            "a reaction entry must name the interrupted mover, not the reactor or nothing",
        );
    }

    // Strictly increasing sequence numbers — the entries are ORDERED, not simultaneous.
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

/// **T2 — C10's greedy-pairing cure.** One shooter that declares TWICE in ONE tick must
/// have each declaration own exactly its OWN rounds.
///
/// A "take the leading run of rounds whose shooter matches this declaration" pairing gives
/// the FIRST declaration every round of the tick and leaves the second with none. That is
/// not a hypothetical shape: the reaction trigger documents a reactor evaluated more than
/// once in a single pass, and the per-turn interrupt cap is at least 2 for a Reactions-2
/// ganger — so one shooter owning two declarations in one tick is exactly the multi-reaction
/// case this ticket exists to make readable.
///
/// The geometry here drives the hazard through the REAL `dispatch_fire` path with two
/// `FireRequested` in one tick, which is the same drain the reaction trigger's interrupts
/// arrive on and is exactly controllable. The cure is that each declaration carries the
/// round count its OWN volley emitted, so the recorder consumes exactly that many from the
/// volley-ordered stream.
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
    // Make TWO shots affordable in ONE tick. A single shot's TU cost scales with the
    // shooter's TU ceiling, so a high ceiling makes one shot cost most of the pool and the
    // second silently resolve to nothing — which would leave the pairing hazard
    // unexercised. Lowering the ceiling first, then filling the pool and the magazine,
    // makes both volleys genuinely fire.
    set_tu_max(&mut app, shooter, 40);
    set_tu(&mut app, shooter, u8::MAX);
    load_magazine(&mut app, shooter, 12);

    // TWO fire requests drained by ONE `dispatch_fire` pass — two declarations, each with
    // its own volley, on one tick.
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

    // For each declaration, count the rounds recorded IMMEDIATELY after it for the same
    // actor. That run is the pairing, and it is what a greedy rule gets wrong.
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
    // The target really was shot at (the fixture is live, not a no-op).
    assert!(
        !logged_of(&app, "RoundResolved").is_empty(),
        "the fixture must actually resolve rounds; target {target:?}",
    );
}

/// The shooter's `single`-mode spec, resolved off its wielded ranged weapon — the SAME
/// source `dispatch_fire` reads, so the fixture arms exactly what the sim will fire.
fn single_mode_of(
    app: &mut bevy::app::App,
    shooter: bevy::prelude::Entity,
) -> gdtf_battle_sim::weapon::FireModeSpec {
    use bevy::ecs::relationship::RelationshipTarget as _;
    use gdtf_battle_sim::weapon::{FireMode, Wields};

    let world = app.world_mut();
    let wielded: Vec<bevy::prelude::Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    let mode = wielded
        .into_iter()
        .find_map(|entity| world.get::<FireMode>(entity).map(FireMode::single));
    let Some(mode) = mode else {
        unreachable!("the shooter wields a ranged weapon carrying a FireMode");
    };
    mode
}

/// Load the shooter's wielded ranged weapon with `rounds` (raising its capacity to match) —
/// the direct test-body mutator, so a fixture can fire more than once in one tick.
fn load_magazine(app: &mut bevy::app::App, shooter: bevy::prelude::Entity, rounds: u16) {
    use bevy::ecs::relationship::RelationshipTarget as _;
    use gdtf_battle_sim::{
        magazine::{LoadedRounds, Magazine},
        weapon::{MagazineSize, Wields},
    };

    let world = app.world_mut();
    let wielded: Vec<bevy::prelude::Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    for entity in wielded {
        let Some(existing) = world.get::<Magazine>(entity).copied() else {
            continue;
        };
        let Some(mut magazine) = world.get_mut::<Magazine>(entity) else {
            continue;
        };
        *magazine = Magazine::new(
            LoadedRounds::new(rounds),
            MagazineSize::new(rounds),
            existing.reload_tu(),
        );
    }
}

/// Lower `entity`'s TU CEILING — the direct test-body mutator. A shot's TU cost is derived
/// from this ceiling, so it is the knob that decides how many acts fit in one turn.
fn set_tu_max(app: &mut bevy::app::App, entity: bevy::prelude::Entity, value: u8) {
    let Some(mut tu_max) = app
        .world_mut()
        .get_mut::<gdtf_battle_sim::ganger::TuMax>(entity)
    else {
        unreachable!("the fixture ganger carries a TuMax ceiling");
    };
    *tu_max = gdtf_battle_sim::ganger::TuMax::new(value);
}
