//! Tests for the battle resources + lifecycle message payloads: the
//! [`PlayerFaction`](crate::battle::PlayerFaction) newtype (construct + Deref,
//! authored-override seed) and the owned-payload contract of the lifecycle
//! messages.

use super::support::*;

// === GTW-226 AC1 — PlayerFaction is a public newtype Resource over Faction with
// new() + a derived Deref reading the inner Faction back. ===

#[test]
fn player_faction_constructs_and_derefs_to_its_inner_faction() {
    let player = PlayerFaction::new(Faction::new(2));
    assert_eq!(
        *player,
        Faction::new(2),
        "PlayerFaction::new(Faction(2)) must Deref back to Faction(2)",
    );
}

// === GTW-226 AC3 — an authored player_faction:1 overrides the Faction(0) default,
// proving the seed reads situation.player_faction, not a hardcoded 0. ===

#[test]
fn authored_player_faction_overrides_the_default_seed() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation_player_faction_one(),
        BattleSeed::new(SEED),
    ));
    app.update();

    let player = app.world().get_resource::<PlayerFaction>().copied();
    assert_eq!(
        player,
        Some(PlayerFaction::new(Faction::new(1))),
        "an authored player_faction:1 must seed PlayerFaction(Faction(1)), not the default 0",
    );
}

/// The lifecycle messages are constructible and carry their payload (the no-bare
/// payload contract): `SetupBattleRequested` owns a `Situation` + `BattleSeed`
/// with no lifetime parameter (that this names `SetupBattleRequested` without
/// `<'_>` is the proof).
#[test]
fn lifecycle_messages_carry_their_payload() {
    let seed = BattleSeed::new(SEED);
    let setup: SetupBattleRequested = SetupBattleRequested::new(two_ganger_situation(), seed);
    assert_eq!(setup.seed, seed, "SetupBattleRequested carries the seed");
    assert_eq!(
        setup.situation.gangers.len(),
        2,
        "SetupBattleRequested carries the owned Situation",
    );
    // The two signals are unit payloads (their identity IS the message — nothing to
    // carry); constructing them and comparing against a fresh value proves they are
    // the zero-field markers the app sends / reads.
    let teardown = TeardownBattleRequested;
    assert_eq!(
        Some(teardown),
        Some(TeardownBattleRequested),
        "TeardownBattleRequested is a unit signal",
    );
    let ready = BattleReady;
    assert_eq!(
        Some(ready),
        Some(BattleReady),
        "BattleReady is a unit signal",
    );
}
