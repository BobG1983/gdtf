//! The end-to-end pacing proof for the GTW-461 enemy-act cadence — driven through the REAL
//! systems in a headless [`SimActsPlugin`](crate::acts::SimActsPlugin) app (the real
//! [`enemy_ai_turn`](crate::ai::enemy_ai_turn) + the live `dispatch_fire`/`dispatch_move`
//! over many `app.update()` ticks; no mocks).
//!
//! These tests assert the contract clauses C1/C2/C4/C5: the brain emits AT MOST ONE enemy
//! act per cadence-step (C1), so the enemy turn resolves act-by-act rather than as a
//! one-frame volley (C2); the turn still terminates and hands control back (C4); and the
//! pacing is deterministic + PIN-DISCRIMINATING — it FAILS if the cadence gate is removed,
//! because acts would then land all-at-once (C5).

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, cooldown_ticks, drain_fires, ground, place_occupant,
    set_cadence, spawn_combatant,
};
use crate::ganger::Direction;

/// A real positive test cadence (ticks between enemy acts). Small enough to keep the test
/// fast, large enough that "gated" vs "all-at-once" is unambiguous: with the gate present,
/// after the first act the next act cannot land for `CADENCE` more ticks; without the gate,
/// the second act would land the very next tick.
const CADENCE: u32 = 8;

/// A generous frame cap — the enemy turn is finite (finite TU + ammo, every act spends TU),
/// so even paced it must return control to the player well within this.
const FRAME_CAP: usize = 400;

/// Stand two enemies that can both see + fire one player on a clear west→east lane, each with
/// a full TU pool + ammo for several shots, plus the player target as a placed HIGH-band
/// occupant the LOS march can strike. Returns the spawned `app`.
///
/// Both enemies at `x = 2` / `x = 3` face East at the player at `(8,5)`; clear LOS down
/// `y = 5`, in view range, in arc — so each is engageable and the brain has MORE than one
/// act's worth of work, the whole point of a pacing test.
fn two_enemies_engaging_one_player() -> bevy::prelude::App {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    // Two acting enemies, both engageable, both with ammo + TU for several shots.
    let _e1 = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let _e2 = spawn_combatant(
        app.world_mut(),
        ground(3, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    app
}

/// C1/C2/C5 — the brain emits AT MOST ONE enemy act per cadence-step, so the enemy volley
/// resolves act-by-act and NOT all in one frame.
///
/// PIN-DISCRIMINATING: with the cadence gate present, after the FIRST act lands (frame 1) the
/// SECOND act cannot land until `CADENCE` more ticks elapse — so the per-act emission count
/// stays at `1` across frames `2..=CADENCE` and only reaches `2` at frame `CADENCE + 1`. If
/// the gate were removed (the `EnemyActCooldown` early-return deleted), the second enemy would
/// act the very next tick and the count would already be `2` at frame `2` — flipping the
/// "still one act after CADENCE/2 ticks" assert red.
#[test]
fn the_brain_emits_at_most_one_act_per_cadence_step() {
    let mut app = two_enemies_engaging_one_player();
    set_cadence(&mut app, CADENCE);

    // Frame 1: exactly ONE act (the first enemy's first shot), and the cooldown recharges.
    app.update();
    let first = drain_fires(&mut app);
    assert_eq!(
        first.len(),
        1,
        "exactly one enemy act must land on the first frame (one act per cadence-step): {first:?}",
    );
    assert_eq!(
        cooldown_ticks(&app),
        CADENCE,
        "the cooldown must recharge to the cadence right after an act is emitted",
    );

    // Frames 2..=CADENCE/2: the gate HOLDS — zero further acts while the cooldown counts
    // down. (This is the pin-discriminator: without the gate the second act would land here.)
    let mut total = first.len();
    for _ in 0..(CADENCE / 2) {
        app.update();
        let acts = drain_fires(&mut app);
        assert!(
            acts.is_empty(),
            "no further act may land before the cadence elapses (the gate holds): {acts:?}",
        );
        total += acts.len();
    }
    assert_eq!(
        total, 1,
        "after the first act, the count must still be exactly 1 mid-cadence (gated)",
    );

    // Drive the rest of the cadence-step: the SECOND act lands once the cooldown reaches 0,
    // proving the turn advances act-by-act rather than all-at-once.
    let mut acted_again = false;
    for _ in 0..=CADENCE {
        app.update();
        if !drain_fires(&mut app).is_empty() {
            acted_again = true;
            break;
        }
    }
    assert!(
        acted_again,
        "a SECOND act must land after the cadence elapses (the turn advances act-by-act)",
    );
}

/// C4 — even paced, the enemy turn TERMINATES and hands control back to the player; every act
/// is a real one (the per-act fire count is bounded by the enemies' ammo/TU, never a runaway).
#[test]
fn the_paced_enemy_turn_terminates_and_hands_control_back() {
    let mut app = two_enemies_engaging_one_player();
    set_cadence(&mut app, CADENCE);

    let mut fires = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        returned,
        "the paced enemy turn must terminate and hand control back to the player within the cap",
    );
    assert!(
        !fires.is_empty(),
        "the enemies must have fired at least one real shot during the paced turn",
    );
}

/// C4 (determinism) — under a fixed seed, the paced turn is reproducible: two runs of the
/// IDENTICAL paced scenario emit the IDENTICAL sequence of fire acts (same shooters, same
/// target cells, same order). Pacing changes WHEN acts land, never WHICH acts land.
#[test]
fn the_paced_turn_is_deterministic_under_a_seeded_rng() {
    let run = || {
        let mut app = two_enemies_engaging_one_player();
        set_cadence(&mut app, CADENCE);
        let mut fires = Vec::new();
        for _ in 0..FRAME_CAP {
            app.update();
            for fire in drain_fires(&mut app) {
                fires.push((fire.shooter, fire.target_cell, *fire.target_level));
            }
            if active_of(&app) == PLAYER {
                break;
            }
        }
        fires
    };

    let first = run();
    let second = run();
    assert!(
        !first.is_empty(),
        "the paced turn must emit at least one act (else the determinism check is vacuous)",
    );
    assert_eq!(
        first, second,
        "the paced enemy turn must emit the identical fire sequence across two seeded runs",
    );
}
