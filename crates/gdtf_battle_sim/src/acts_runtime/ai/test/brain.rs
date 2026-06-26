//! The end-to-end integration proof for the enemy brain (GTW-70) — driven through the REAL
//! systems in a headless [`SimActsPlugin`] app (no mocks): an enemy that can SEE a player
//! FIRES it (the real fire path) and the turn passes back to the player; an enemy that
//! CANNOT see a player ADVANCES toward contact (the real move path); and the enemy turn
//! always terminates within a bounded number of frames, spending TU within budget.

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_fires, drain_moves, give_disabled_hand, ground,
    place_occupant, spawn_combatant, spawn_combatant_handed, tu_of,
};
use crate::{armor::BodyPart, ganger::Direction, metric::Cell, weapon::Handedness};

/// A generous frame cap — the enemy turn is finite (TU + ammo are finite, every act spends
/// TU or attaches a bounded walk), so it must return control to the player well within this.
const FRAME_CAP: usize = 80;

/// Scenario 1 — an enemy that can SEE a player FIRES it (the real fire path) and the turn
/// passes back to the player. The enemy at (2,5) faces East at the player occupant at (8,5);
/// clear LOS, in view range, in arc → it emits a real [`FireRequested`] at the player that
/// `dispatch_fire` resolves. Once the enemy has nothing left to do it ends its turn, handing
/// control back to the player; its TU is spent within its budget and the loop terminates.
#[test]
fn enemy_that_can_see_fires_then_the_turn_returns_to_the_player() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        2,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    // The player is a grid occupant (HIGH band) so the LOS march can strike it.
    place_occupant(&mut app, player_at, player);

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

    // The enemy fired at the player through the REAL fire path (the brain emitted a
    // FireRequested the live dispatch_fire processed) — aimed at the player's (8,5,0) cell.
    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "the enemy must emit a FireRequested at the player's (8,5,0) cell: {fires:?}",
    );
    assert!(
        returned,
        "the enemy turn must terminate and hand control back to the player within the cap",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU within its budget (it fired / acted): {}",
        tu_of(&app, enemy),
    );
}

/// Scenario 2 — an enemy that CANNOT see a player ADVANCES toward contact (the real move
/// path). The enemy at (2,5) with a small TU pool and the player far west-of-range at (40,5):
/// out of view range, so no fire — instead the brain emits a real [`MoveRequested`] toward
/// the player that `dispatch_move` accepts, the enemy walks east (closer), no shot is ever
/// fired, and the turn terminates back to the player with TU spent.
#[test]
fn enemy_that_cannot_see_advances_toward_contact() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    // A small TU pool so the advance is a few cells and the turn ends quickly.
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut moves = Vec::new();
    let mut fires = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        moves.extend(drain_moves(&mut app));
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    // The enemy advanced toward the player — a MoveRequested whose destination is EAST of
    // the enemy's start (closer to the player at x=40).
    assert!(
        moves.iter().any(|m| m.actor == enemy && m.dest.x > 2),
        "the enemy must emit a MoveRequested stepping toward the player (east of x=2): {moves:?}",
    );
    // It can never SEE the player (out of range), so it never fires.
    assert!(
        fires.is_empty(),
        "an enemy that cannot see the player must never fire: {fires:?}",
    );
    assert!(
        returned,
        "the advance turn must terminate and hand control back to the player within the cap",
    );
    assert!(
        tu_of(&app, enemy) < 20,
        "the enemy spent TU advancing within its budget: {}",
        tu_of(&app, enemy),
    );
}

/// Scenario 3 — FACTION CORRECTNESS, target half (GTW-70 §A): the brain fires the OPPOSING
/// player and REFUSES to target a friendly enemy. On the ENEMY turn, an acting enemy at (2,5)
/// faces East with a friendly enemy at (4,3) — NEARER, in arc + range, a placed occupant the
/// LOS march can strike — and an opposing player at (8,5) further East (also in arc + range,
/// a placed occupant). The brain splits FIRE targets on `faction != active`, so the friendly
/// enemy is never a target: the enemy fires the PLAYER, never the teammate.
///
/// Pin-discriminating against the target filter (`brain.rs` `row.faction != active_faction`):
/// were it reverted to `==`, the FIRE target set would become the enemy's own faction, the
/// NEARER friendly enemy would be picked, and BOTH asserts below flip red — the enemy would
/// fire at (4,3) and never at the player. The positive `shooter == enemy` / `target_cell ==`
/// pair is EXACT-entity / EXACT-cell equality, and the friendly-fire guard is an exact-cell
/// non-membership — not a loose `.any(|f| f.shooter == enemy)` that a stray player shot slips.
#[test]
fn enemy_fires_the_opposing_player_and_never_a_friendly_enemy() {
    let mut app = brain_app();
    let friend_at = ground(4, 3);
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    // A friendly enemy NEARER than the player, in the acting enemy's East arc + range, and a
    // placed HIGH-band occupant — so under a reverted target filter it would BOTH pass the LOS
    // march AND win `pick_nearest`. The brain must still refuse to target it (same faction).
    let friend = spawn_combatant(app.world_mut(), friend_at, ENEMY, Direction::East, 100, 6);
    place_occupant(&mut app, friend_at, friend);
    // The opposing player target the enemy can legitimately engage (clear LOS down y=5).
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    // TARGET-ONLY-OPPOSING: the acting enemy aimed its real FireRequested at the PLAYER's exact
    // (8,5,0) cell.
    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "the enemy must fire the opposing player at its (8,5,0) cell: {fires:?}",
    );
    // NO FRIENDLY FIRE: no shot — from ANY shooter — is ever aimed at the friendly enemy's
    // (4,3) cell. Flips red the instant the target filter is reverted to `== active_faction`.
    assert!(
        !fires.iter().any(|f| f.target_cell == Cell::new(4, 3)),
        "the brain must never target a friendly enemy (no shot at (4,3)): {fires:?}",
    );
    // Corollary: the friendly enemy is the teammate's entity, never the fire target's ref.
    assert_ne!(
        friend, player,
        "the friendly enemy and the player must be distinct entities",
    );
}

/// Scenario 4 — FACTION CORRECTNESS, actor half (GTW-70 §A): the brain drives ONLY enemy
/// gangers — never the player's units — on the enemy turn. On the ENEMY turn, the acting enemy
/// at (2,5) faces East at an opposing player at (6,5) it engages; a SECOND player stands at
/// (8,5) facing West (behind the first, so the enemy never sees it, but in the first player's
/// arc + range). The brain splits ACTORS on `faction == active`, so neither player ever acts.
///
/// Pin-discriminating against the actor filter (`brain.rs` `row.faction == active_faction`):
/// were it reverted to `!=`, the ACTING set would become the players, and the (8,5) player —
/// facing the (6,5) player in its arc with clear LOS — would emit a `FireRequested` whose
/// `shooter` is a PLAYER entity, flipping the DRIVE-ONLY-ENEMIES asserts red. The asserts use
/// EXACT-entity equality on `shooter` / `actor` (non-membership), not a loose `.any`.
#[test]
fn enemy_brain_never_drives_a_player_unit() {
    let mut app = brain_app();
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    // The acting enemy's legitimate opposing target (clear LOS down y=5).
    let player_near_at = ground(6, 5);
    let player_near = spawn_combatant(
        app.world_mut(),
        player_near_at,
        PLAYER,
        Direction::West,
        100,
        6,
    );
    place_occupant(&mut app, player_near_at, player_near);
    // A second player behind the first (the enemy's LOS is blocked by `player_near`), facing
    // West so that — were the brain wrongly to drive players — it could SEE and FIRE the first
    // player. Under the correct actor filter it must stay inert on the enemy turn.
    let player_far_at = ground(8, 5);
    let player_far = spawn_combatant(
        app.world_mut(),
        player_far_at,
        PLAYER,
        Direction::West,
        100,
        6,
    );
    place_occupant(&mut app, player_far_at, player_far);

    let mut fires = Vec::new();
    let mut moves = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        moves.extend(drain_moves(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    // DRIVE-ONLY-ENEMIES (the discriminating assert): neither player is ever made a shooter.
    // Flips red the instant the actor filter is reverted to `!= active_faction` (the players
    // then become the acting set and `player_far` fires `player_near`).
    assert!(
        !fires
            .iter()
            .any(|f| f.shooter == player_near || f.shooter == player_far),
        "the brain must never drive a player as a shooter on the enemy turn: {fires:?}",
    );
    // DRIVE-ONLY-ENEMIES: neither player is ever made a mover either.
    assert!(
        !moves
            .iter()
            .any(|m| m.actor == player_near || m.actor == player_far),
        "the brain must never drive a player as a mover on the enemy turn: {moves:?}",
    );
    // Liveness: the brain actually ran the real fire path — the ENEMY engaged the nearer
    // player at its (6,5,0) cell (proves the scenario is live, not inert).
    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(6, 5)
            && *f.target_level == 0),
        "the enemy must engage the nearer player at (6,5,0): {fires:?}",
    );
}

/// GTW-443 C7 — AI PARITY: the AI brain (which runs the SHARED `can_fire`) does NOT
/// engage with a `TwoHanded` weapon when the shooter has fewer than two hands. The enemy at
/// (2,5) faces East at a player occupant at (8,5) — clear LOS, in arc + range, the IDENTICAL
/// geometry that fires in the positive control below — but wields a `TwoHanded` weapon AND
/// carries a hand-disabling injury (one hand left), so the shared fire guard refuses it and
/// the brain NEVER emits a `FireRequested` at the player.
///
/// This sits on the BRAIN path (not the player surface) on purpose: it distinguishes the
/// shared-guard siting (a single gate the AI and player both fold their hand count into)
/// from a player-only gate that would leave the AI ungated. Pin-discriminating: the
/// `two_handed_at_two_hands` control proves the same geometry DOES fire, so the absence of
/// fire here is the hand-count gate, not a dead scenario.
#[test]
fn ai_does_not_engage_two_handed_weapon_below_two_hands() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant_handed(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        Handedness::TwoHanded,
    );
    // A hand-disabling injury drops the enemy to one hand → the 2H weapon is refused.
    give_disabled_hand(app.world_mut(), enemy, BodyPart::RightArm);
    let player = spawn_combatant_handed(
        app.world_mut(),
        player_at,
        PLAYER,
        Direction::West,
        Handedness::OneHanded,
    );
    place_occupant(&mut app, player_at, player);

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

    // The AI never fires the two-handed weapon at one hand (the shared can_fire refuses it).
    assert!(
        fires.is_empty(),
        "the AI must NOT engage a TwoHanded weapon below two hands (shared can_fire gate): {fires:?}",
    );
    // The turn still terminates (the brain advances/holds, then ends the turn).
    assert!(
        returned,
        "the enemy turn must still terminate within the cap (advance/hold, then end-turn)",
    );
}

/// GTW-443 C7 positive control — the SAME engagement geometry DOES fire when the enemy
/// has both hands for its `TwoHanded` weapon, proving the no-fire above is the hand-count
/// gate and not an inert scenario (the gate is conditional, not a blanket 2H ban).
#[test]
fn ai_engages_two_handed_weapon_with_two_hands() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant_handed(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        Handedness::TwoHanded,
    );
    // No disabling injury → both hands → the 2H weapon is usable.
    let player = spawn_combatant_handed(
        app.world_mut(),
        player_at,
        PLAYER,
        Direction::West,
        Handedness::OneHanded,
    );
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "with two hands the AI engages the TwoHanded weapon at the player's (8,5,0) cell: {fires:?}",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU firing: {}",
        tu_of(&app, enemy),
    );
}
