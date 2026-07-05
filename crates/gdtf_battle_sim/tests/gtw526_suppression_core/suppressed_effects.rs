//! What suppression does to the suppressed unit: zero reaction draws, clearing at
//! its own turn start, and the auto-stance drop behind cover (clauses (b) / (c) / (d)).

use bevy::app::App;
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, StanceKind, Suppressed,
    acts::{EndTurnRequested, FireRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, Destroyed, HeightBand},
    ganger::Direction,
    metric::{Cell, CellLevel, Level},
    rng::ReactionRng,
    test_support::{SituationBuilder, reaction_rng},
};

use super::harness::*;

/// Run `app.update()` until the active faction is `faction` (crossing turn boundaries as
/// the enemy AI ends its turn back), or a generous tick budget is exhausted. Bounded.
fn run_until_active(app: &mut App, faction: u8) {
    for _ in 0..96 {
        app.update();
        if active_faction_is(app, faction) {
            return;
        }
    }
}

/// A cover prototype at the given `band` (the test terrain registry ships only a LOW
/// cover and a HIGH wall, so a MID-band piece is seeded directly into the ledger from the
/// test body).
const fn cover_at_band(band: HeightBand) -> CoverEntry {
    CoverEntry {
        current_hp:       CoverHp::new(30),
        max_hp:           CoverHp::new(30),
        height_band:      band,
        armor_protection: ArmorProtection::new(2),
        armor_hardness:   ArmorHardness::new(1),
        destroyed:        Destroyed::new(false),
    }
}

/// A MID-band cover prototype seeded directly into the ledger.
const fn mid_cover() -> CoverEntry {
    cover_at_band(HeightBand::Mid)
}

/// A LOW-band cover prototype seeded directly into the ledger.
const fn low_cover() -> CoverEntry {
    cover_at_band(HeightBand::Low)
}

/// Seed a cover entry into the battle's `CoverLedger` at `cell` (test-body idiom).
fn seed_cover(app: &mut App, cell: CellLevel, entry: CoverEntry) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("BattleSimPlugin inserts the CoverLedger at setup");
    };
    ledger.insert(cell, entry);
}

// === (b) — a suppressed reactor does NOT interrupt AND consumes ZERO ReactionRng draws;
// an unsuppressed control DOES interrupt and advances the stream. ===

#[test]
fn suppressed_reactor_consumes_zero_reaction_rng_draws() {
    // The first draw of a FRESH ReactionRng from this seed — the position the world's
    // stream must still sit at when the suppressed reactor draws nothing.
    let fresh_first_draw = reaction_rng(SEED).next_u64();

    // Build a scenario where a ganger FIRING trips the FireDeclaration act-in-LOS surface,
    // and an opposing watcher in LOS would interrupt (forced p == 1.0). One reactor, one
    // actor — so the ONLY possible ReactionRng draw is the watcher's interrupt roll.
    let build = |suppress_watcher: bool| {
        // Radius 0 so the actor's own fire suppresses only its target cell (an empty cell),
        // never the watcher — the watcher's suppression is set explicitly below, isolating
        // the C3 gate from the producer.
        let mut app = battle_app(0);
        let situation = SituationBuilder::new()
            .with_gangers([
                // The ACTOR: an enemy at (6,5) firing east (away from the watcher) — the
                // FireDeclaration is the act the watcher reacts to.
                ganger(ground(6, 5), ENEMY, Direction::East),
                // The WATCHER: a player at (4,5) facing east, in LOS + arc + range of the
                // actor's cell.
                ganger(ground(4, 5), PLAYER, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(actor), Some(watcher)) =
            (ganger_of(&mut app, ENEMY), ganger_of(&mut app, PLAYER))
        else {
            unreachable!("one enemy actor + one player watcher spawned");
        };
        if suppress_watcher {
            // Suppress the WATCHER directly (test-body idiom) BEFORE it can react — the C3
            // reaction-gate must skip it with zero RNG draws.
            app.world_mut().entity_mut(watcher).insert(Suppressed::new(
                gdtf_battle_sim::ganger::SuppressorCell::new(ground(6, 5)),
            ));
            step(&mut app, 1); // no-op stance drop (no cover), keeps the marker
        }
        // The actor FIRES (a real FireRequested at a far cell, away from the watcher) — this
        // trips the FireDeclaration surface reaction_trigger reads next tick.
        let mode = wielded_single_mode(&mut app, actor);
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(9, 5),
            Level::new(0),
        ));
        step(&mut app, 4);
        let watcher_tu = tu_of(&app, watcher);
        // The world's ReactionRng position — its NEXT draw (mutates the world's stream, but
        // the app is discarded right after, so it is a safe read of the current position).
        let next_draw = app
            .world_mut()
            .get_resource_mut::<ReactionRng>()
            .map(|mut r| r.next_u64());
        (watcher, watcher_tu, next_draw)
    };

    // CONTROL: watcher NOT suppressed → it interrupts → 1 ReactionRng draw consumed → the
    // stream has ADVANCED past the fresh first draw.
    let (_control_watcher, control_tu, control_next) = build(false);
    let Some(control_tu) = control_tu else {
        unreachable!("the control watcher persists");
    };
    let Some(control_next) = control_next else {
        unreachable!("the ReactionRng stream is present in the control");
    };

    // SUPPRESSED: watcher IS suppressed → it does NOT interrupt → 0 ReactionRng draws → the
    // stream still sits at its INITIAL position (its next draw == the fresh first draw).
    let (_supp_watcher, supp_tu, supp_next) = build(true);
    let Some(supp_tu) = supp_tu else {
        unreachable!("the suppressed watcher persists");
    };
    let Some(supp_next) = supp_next else {
        unreachable!("the ReactionRng stream is present in the suppressed run");
    };

    assert_eq!(
        supp_next, fresh_first_draw,
        "(b) the suppressed reactor consumed ZERO ReactionRng draws — the world stream still \
         sits at its initial position (next draw == a fresh stream's first draw)",
    );
    assert_ne!(
        control_next, fresh_first_draw,
        "(b) precondition: the unsuppressed control reactor DID draw (its interrupt roll \
         advanced the stream past the fresh first draw) — else the zero-draw claim is vacuous",
    );
    // Structural corroboration through TU: the control watcher spent fire TU on its
    // interrupt; the suppressed watcher did not interrupt, so its TU is untouched.
    assert!(
        control_tu < supp_tu,
        "(b) the control watcher's TU debits on its interrupt shot ({control_tu}); the \
         suppressed watcher never fires, so its TU is higher ({supp_tu})",
    );
}

// === (c) — the clear cadence: a unit stays suppressed through the opponent's turn and
// clears at its OWN faction's TurnStarted. ===

#[test]
fn suppression_clears_at_the_suppressed_units_own_turn_start() {
    let mut app = battle_app(1);
    // A player unit at the target of an enemy shot (player turn is active at setup, but we
    // drive the shot directly). The enemy shooter is west.
    let target = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(target, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("one player + one enemy spawned");
    };

    // Suppress the PLAYER via a real enemy fire on its cell.
    let mode = wielded_single_mode(&mut app, enemy);
    app.world_mut().write_message(FireRequested::new(
        enemy,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    assert!(
        is_suppressed(&app, player),
        "(c) precondition: the player is suppressed by the enemy shot",
    );

    // End the PLAYER turn → the ENEMY turn begins (a TurnStarted { now_active: ENEMY }). The
    // player unit must STAY suppressed — it is now in its opponent's turn.
    app.world_mut().write_message(EndTurnRequested);
    step(&mut app, 3);
    assert!(
        active_faction_is(&app, ENEMY),
        "(c) precondition: the enemy turn is active after the player ends its turn",
    );
    assert!(
        is_suppressed(&app, player),
        "(c) the player stays suppressed through the ENEMY's turn (cleared only at its OWN \
         turn-start, not the opponent's)",
    );

    // The enemy AI runs its turn and ends it back to the player → a TurnStarted { now_active:
    // PLAYER }. The player unit's suppression must CLEAR at that own-faction boundary.
    run_until_active(&mut app, PLAYER);
    assert!(
        active_faction_is(&app, PLAYER),
        "(c) precondition: control cycled back to the player turn",
    );
    // A settle tick so the same-frame reset_suppression (.after(dispatch_end_turn)) has run.
    step(&mut app, 1);
    assert!(
        !is_suppressed(&app, player),
        "(c) the player's suppression clears at ITS OWN (player) turn-start",
    );
}

// === (d) — auto-stance: low cover → Prone, mid cover → Crouching, no cover → unchanged;
// NO TU charged. ===

#[test]
fn auto_stance_drops_behind_cover_without_charging_tu() {
    // Three separate suppressions, each in its own app so the cover geometry is clean.
    // The suppressor sits WEST of the unit, so the cover "one step toward the suppressor"
    // is one cell WEST of the unit.
    let run = |cover: Option<CoverEntry>| -> (Option<StanceKind>, Option<u8>) {
        let mut app = battle_app(0);
        let unit_cell = ground(8, 5);
        let cover_cell = ground(7, 5); // one step WEST (toward the suppressor at (4,5))
        let situation = SituationBuilder::new()
            .with_gangers([
                ganger(unit_cell, PLAYER, Direction::East),
                ganger(ground(4, 5), ENEMY, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        if let Some(entry) = cover {
            seed_cover(&mut app, cover_cell, entry);
        }
        let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("one player + one enemy spawned");
        };
        let tu_before = tu_of(&app, player);
        // Enemy fires on the player's cell → suppresses it (radius 0 = the target cell).
        let mode = wielded_single_mode(&mut app, enemy);
        app.world_mut().write_message(FireRequested::new(
            enemy,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        assert!(
            is_suppressed(&app, player),
            "auto-stance precondition: the player is suppressed",
        );
        let stance_after = stance_of(&app, player);
        let tu_after = tu_of(&app, player);
        // The auto-stance drop must NOT charge TU (a reflexive duck, no action spent).
        assert_eq!(
            tu_before, tu_after,
            "(d) the auto-stance drop charges NO TU (before {tu_before:?} == after \
             {tu_after:?})",
        );
        (stance_after, tu_after)
    };

    let (low_stance, _) = run(Some(low_cover()));
    assert_eq!(
        low_stance,
        Some(StanceKind::Prone),
        "(d) LOW cover → the unit auto-drops to Prone",
    );

    let (mid_stance, _) = run(Some(mid_cover()));
    assert_eq!(
        mid_stance,
        Some(StanceKind::Crouching),
        "(d) MID cover → the unit auto-drops to Crouching",
    );

    let (none_stance, _) = run(None);
    assert_eq!(
        none_stance,
        Some(StanceKind::Standing),
        "(d) NO adjacent cover → the stance is UNCHANGED (still Standing, the spawn posture)",
    );
}
