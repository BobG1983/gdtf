//! Census-driven win/loss (a faction wiped by downing) ends the battle (GTW-239 AC5).

use gdtf_app::test_support::BattleScapeState;
use gdtf_battle_sim::ganger::{Faction, LifeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// Set the [`LifeState`] of every spawned ganger whose [`Faction`] is `faction` to `to`, via a
/// `world_mut()` query in the TEST BODY (`bevy-traps.md` #7 carve-out (a) — NOT a registered
/// system or a helper taking `&mut World`). The accepted way to drive a faction out of the
/// fight so the REAL `check_outcome` census emits an outcome, without re-running the damage
/// pipeline. Mirrors the sim's in-test `set_faction_life_state`.
fn down_faction(app: &mut bevy::app::App, faction: u8, to: LifeState) {
    let target = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &mut LifeState)>();
    for (&fac, mut life) in query.iter_mut(world) {
        if fac == target {
            *life = to;
        }
    }
}

/// AC5(a) — FULL census → outcome → app ends the battle (WIN).
///
/// The end-to-end integration over the REAL GTW-237 `check_outcome` census (no hand-written
/// message): drives into `BattleRunning` with the two-ganger fixture (player faction defaults to
/// gang 0; the enemy is gang 1), then sets the ENEMY gang (faction 1) `Dead` while the player
/// gang (faction 0) stays `Alive`. The sim's `check_outcome` (in the gated `Simulate` band) then
/// emits `BattleWon`, the app's `end_battle_on_outcome` reads it `.after(Simulate)` the SAME
/// update and inserts the marker, and `move_on` advances out of `BattleRunning`. Asserts
/// `AnimateOut`. This is the strongest evidence: the sim-signal → app-lifecycle path for a WIN.
#[test]
fn census_win_ends_the_battle_to_animate_out() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Drive the REAL census: every ENEMY (faction 1, non-player) is out of the fight, the
    // player (faction 0) still stands → check_outcome emits BattleWon.
    down_faction(&mut app, TARGET_FACTION, LifeState::Dead);

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "the real census win (all enemies Dead, player Alive) must end the battle within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a census-emitted BattleWon must advance BattleRunning → AnimateOut end-to-end",
    );
}

/// AC5(b) — FULL census → outcome → app ends the battle (LOSS).
///
/// The loss twin of [`census_win_ends_the_battle_to_animate_out`]: sets every PLAYER ganger
/// (faction 0) out of the fight so the sim's `check_outcome` emits `BattleLost`, and asserts the
/// app likewise reaches `AnimateOut` via the same chain. Proves the end-to-end
/// sim-signal → app-lifecycle path for a LOSS.
#[test]
fn census_loss_ends_the_battle_to_animate_out() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Drive the REAL census: every PLAYER ganger (faction 0, the default player_faction) is out
    // of the fight → check_outcome emits BattleLost (enemy liveness is irrelevant to a loss).
    down_faction(&mut app, SHOOTER_FACTION, LifeState::Dead);

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "the real census loss (all player gangers Dead) must end the battle within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a census-emitted BattleLost must advance BattleRunning → AnimateOut end-to-end",
    );
}
