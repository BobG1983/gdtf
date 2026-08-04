//! authored-override seed) and the owned-payload contract of the lifecycle
use super::support::*;

// === AC1 — PlayerFaction is a public newtype Resource over Faction with
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
