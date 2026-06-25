//! Tests for the roster-grounded WIN/LOSS outcome census
//! [`check_outcome`](crate::battle::check_outcome) (GTW-237): the win/loss
//! arms, the at-most-once latches, the inert-without-a-battle gate, the
//! roster-grounded (not scan-grounded) existence, and the mutual-wipe = loss rule.

use super::support::*;

// === GTW-237 — the roster-grounded WIN/LOSS outcome census (check_outcome). The
// fixtures go through the REAL setup (SetupBattleRequested seeds PlayerFaction +
// BattleRoster) and drive LifeState via a world_mut() query (bevy-traps #7 carve-out).
// ===

/// AC1 — all enemies down + a live player → exactly ONE `BattleWon`, zero `BattleLost`.
#[test]
fn all_enemies_down_with_live_player_emits_one_battle_won_and_no_loss() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        one_player_two_enemy_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    // Drain the setup-frame census output (no outcome yet — all gangers Alive).
    let _ = drain_battle_won(&mut app);
    let _ = drain_battle_lost(&mut app);

    // One enemy Dead, the other Downed (both count OUT); the faction-0 player stays
    // Alive. The setter only flips Alive gangers, so the two calls down DISTINCT enemies.
    assert!(
        set_one_faction_ganger_life_state(&mut app, 1, LifeState::Dead),
        "precondition: a first enemy ganger to set Dead",
    );
    assert!(
        set_one_faction_ganger_life_state(&mut app, 1, LifeState::Downed),
        "precondition: a second (still-Alive) enemy ganger to set Downed",
    );
    app.update();

    assert_eq!(
        drain_battle_won(&mut app),
        1,
        "all enemies out (Dead + Downed) with a live player must emit exactly one BattleWon",
    );
    assert_eq!(
        drain_battle_lost(&mut app),
        0,
        "a player still standing must emit NO BattleLost",
    );
}

/// AC2 — any enemy still `Alive` → NO `BattleWon`.
#[test]
fn any_enemy_alive_emits_no_battle_won() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        one_player_two_enemy_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    let _ = drain_battle_won(&mut app);

    // Only ONE enemy Dead; the other faction-1 ganger stays Alive.
    assert!(
        set_one_faction_ganger_life_state(&mut app, 1, LifeState::Dead),
        "precondition: the fixture fielded ≥1 enemy ganger to set Dead",
    );
    app.update();

    assert_eq!(
        drain_battle_won(&mut app),
        0,
        "with an enemy still Alive the census must NOT win",
    );
}

/// AC3 — all players down → exactly ONE `BattleLost`, enemies still Alive → no
/// `BattleWon`.
#[test]
fn all_players_down_emits_one_battle_lost_and_no_win() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    let _ = drain_battle_won(&mut app);
    let _ = drain_battle_lost(&mut app);

    // The single player ganger (faction 0) goes Dead; the faction-1 enemy stays Alive.
    set_faction_life_state(&mut app, 0, LifeState::Dead);
    app.update();

    assert_eq!(
        drain_battle_lost(&mut app),
        1,
        "all players out must emit exactly one BattleLost",
    );
    assert_eq!(
        drain_battle_won(&mut app),
        0,
        "an enemy still Alive (and the player wiped) must emit NO BattleWon",
    );
}

/// AC4 — at-most-once per outcome (no per-frame spam): the win latch and the loss
/// latch each suppress every emit after the first.
#[test]
fn outcomes_emit_at_most_once_per_battle() {
    // Win latch: the AC1 all-enemies-down fixture.
    let mut win_app = headless_app();
    win_app.world_mut().write_message(SetupBattleRequested::new(
        one_player_two_enemy_situation(),
        BattleSeed::new(SEED),
    ));
    win_app.update();
    let _ = drain_battle_won(&mut win_app);

    set_faction_life_state(&mut win_app, 1, LifeState::Dead);
    win_app.update();
    assert_eq!(
        drain_battle_won(&mut win_app),
        1,
        "the first all-enemies-down frame emits exactly one BattleWon",
    );
    // Several more frames WITHOUT draining: the latch suppresses every further emit.
    for _ in 0..3 {
        win_app.update();
    }
    assert_eq!(
        drain_battle_won(&mut win_app),
        0,
        "the win latch must suppress all further BattleWon (no per-frame spam)",
    );

    // Loss latch: the AC3 all-players-down fixture.
    let mut loss_app = headless_app();
    loss_app
        .world_mut()
        .write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(SEED),
        ));
    loss_app.update();
    let _ = drain_battle_lost(&mut loss_app);

    set_faction_life_state(&mut loss_app, 0, LifeState::Dead);
    loss_app.update();
    assert_eq!(
        drain_battle_lost(&mut loss_app),
        1,
        "the first all-players-down frame emits exactly one BattleLost",
    );
    for _ in 0..3 {
        loss_app.update();
    }
    assert_eq!(
        drain_battle_lost(&mut loss_app),
        0,
        "the loss latch must suppress all further BattleLost (no per-frame spam)",
    );
}

/// AC5 — inert outside a live battle: no setup means no `BattleInProgress` /
/// `PlayerFaction` / `BattleRoster`, so the Simulate-gated census never runs and both
/// buffers drain to 0.
#[test]
fn census_is_inert_without_a_live_battle() {
    let mut app = headless_app();
    // No SetupBattleRequested — the battle-lifetime resources are all absent.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "precondition: no setup means no BattleInProgress",
    );
    assert!(
        app.world().get_resource::<BattleRoster>().is_none(),
        "precondition: no setup means no BattleRoster",
    );
    assert!(
        app.world().get_resource::<PlayerFaction>().is_none(),
        "precondition: no setup means no PlayerFaction",
    );

    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        drain_battle_won(&mut app),
        0,
        "the Simulate gate keeps check_outcome from running, so NO BattleWon",
    );
    assert_eq!(
        drain_battle_lost(&mut app),
        0,
        "the Simulate gate keeps check_outcome from running, so NO BattleLost",
    );
}

/// AC6(a) — roster-grounded, NOT scan-grounded: a player-only situation (empty enemy
/// roster) never wins, even though every fielded ganger could be "all non-player down"
/// under the old scan (there are no enemies to be down).
#[test]
fn empty_enemy_roster_never_wins() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        player_only_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();

    // has_enemy_of(player) is false for a player-only roster — no false win.
    assert_eq!(
        drain_battle_won(&mut app),
        0,
        "a player-only (empty enemy) roster must never emit BattleWon (has_enemy_of false)",
    );
    // And no loss either: the player gang is fully Alive.
    assert_eq!(
        drain_battle_lost(&mut app),
        0,
        "a fully-Alive player gang must not emit BattleLost",
    );
}

/// AC6(b) — the converse: with enemies fielded and all of them down (corpses persist as
/// entities, the sim has no despawn-on-death), the win STILL fires — existence comes
/// from the roster, so the win is robust regardless of despawn. (Shares the AC1 setup;
/// asserts the roster, not the scan, is what grounds the win.)
#[test]
fn wiped_out_enemy_gang_still_wins_from_the_roster() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        one_player_two_enemy_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    let _ = drain_battle_won(&mut app);

    // The enemy roster IS fielded (the win cannot come from an empty roster) ...
    let roster = app.world().get_resource::<BattleRoster>().cloned();
    assert_eq!(
        roster,
        Some(BattleRoster::new([Faction::new(0), Faction::new(1)])),
        "the roster records both fielded factions",
    );
    // ... and the enemy corpses persist as entities after they go down.
    set_faction_life_state(&mut app, 1, LifeState::Dead);
    let world = app.world_mut();
    let mut enemy_corpses = world.query::<(&Faction, &LifeState)>();
    let dead_enemies = enemy_corpses
        .iter(world)
        .filter(|&(&fac, &life)| fac == Faction::new(1) && life == LifeState::Dead)
        .count();
    assert_eq!(
        dead_enemies, 2,
        "both enemy gangers persist as Dead entities (no despawn)"
    );

    app.update();
    assert_eq!(
        drain_battle_won(&mut app),
        1,
        "with enemies fielded and all down, the roster-grounded win still fires",
    );
}

/// AC7 — `BattleRoster` lifetime tracks `BattleInProgress`: present after a successful
/// setup (its set == the situation's distinct factions), absent after teardown
/// (alongside `BattleInProgress` / `PlayerFaction`).
#[test]
fn battle_roster_lifetime_tracks_battle_in_progress() {
    let mut app = headless_app();

    // ABSENT before any setup.
    assert!(
        app.world().get_resource::<BattleRoster>().is_none(),
        "BattleRoster must be absent before any setup",
    );

    // PRESENT after a successful setup — the set equals the distinct ganger factions.
    app.world_mut().write_message(SetupBattleRequested::new(
        one_player_two_enemy_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    let roster = app.world().get_resource::<BattleRoster>().cloned();
    assert_eq!(
        roster,
        Some(BattleRoster::new([Faction::new(0), Faction::new(1)])),
        "setup must capture a BattleRoster of the situation's distinct ganger factions",
    );

    // ABSENT after a teardown — alongside BattleInProgress + PlayerFaction.
    app.world_mut().write_message(TeardownBattleRequested);
    app.update();
    assert!(
        app.world().get_resource::<BattleRoster>().is_none(),
        "teardown must remove the BattleRoster (lifetime identical to BattleInProgress)",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "teardown must remove BattleInProgress (the shared-lifetime witness)",
    );
    assert!(
        app.world().get_resource::<PlayerFaction>().is_none(),
        "teardown must remove PlayerFaction (the shared-lifetime resource)",
    );
}

/// AC8 — mutual wipe → `BattleLost`, not `BattleWon` (the flagged mutual-exclusion
/// rule): enemies AND players all set OUT in the same update emit one loss, zero wins.
#[test]
fn mutual_wipe_resolves_to_battle_lost_not_won() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        one_player_two_enemy_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    let _ = drain_battle_won(&mut app);
    let _ = drain_battle_lost(&mut app);

    // Everyone falls in the SAME frame: the player gang Dead, the enemy gang Downed.
    set_faction_life_state(&mut app, 0, LifeState::Dead);
    set_faction_life_state(&mut app, 1, LifeState::Downed);
    app.update();

    assert_eq!(
        drain_battle_lost(&mut app),
        1,
        "a mutual wipe must emit exactly one BattleLost",
    );
    assert_eq!(
        drain_battle_won(&mut app),
        0,
        "a mutual wipe must emit NO BattleWon (the win requires a surviving player)",
    );
}
