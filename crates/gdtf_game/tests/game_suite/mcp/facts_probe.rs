use bevy::{
    app::{App, Update},
    ecs::resource::Resource,
    prelude::{Deref, ResMut},
};
use gdtf_game::test_support::{BattleActivity, GameFacts, GameFactsParam, TurnOwner};

use super::{
    battle_setup::battle_with_another_gang_acting,
    socket_support::{
        SocketFixture, TestError, TestResult, battle_app_listening, game_app_listening,
    },
};

#[derive(Resource, Deref, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ObservedFacts(Option<GameFacts>);

fn probe_facts(facts: GameFactsParam, mut observed: ResMut<ObservedFacts>) {
    *observed = ObservedFacts(Some(facts.sample()));
}

fn observed_through(fixture: SocketFixture) -> Result<GameFacts, TestError> {
    let (app, _port) = fixture()?;
    observed_in(app)
}

fn observed_in(mut app: App) -> Result<GameFacts, TestError> {
    app.init_resource::<ObservedFacts>();
    app.add_systems(Update, probe_facts);
    app.update();
    read_observed(&app)
}

fn read_observed(app: &App) -> Result<GameFacts, TestError> {
    match **app.world().resource::<ObservedFacts>() {
        Some(facts) => Ok(facts),
        None => Err("the probe system never ran on the real Update schedule".into()),
    }
}

#[test]
fn the_battle_fixture_samples_a_running_battle_through_the_real_facts_param() -> TestResult {
    assert_eq!(
        observed_through(battle_app_listening)?.battle_activity(),
        BattleActivity::Running,
        "a fixture resting in BattleRunning must sample as a running battle",
    );
    Ok(())
}

#[test]
fn the_menu_fixture_samples_no_running_battle() -> TestResult {
    assert_eq!(
        observed_through(game_app_listening)?.battle_activity(),
        BattleActivity::NotRunning,
        "a fixture resting at the menu has no battle to act in",
    );
    Ok(())
}

#[test]
fn the_battle_fixture_samples_the_players_own_turn() -> TestResult {
    assert_eq!(
        observed_through(battle_app_listening)?.turn_owner(),
        TurnOwner::Player,
        "a fresh battle opens on the player's turn, so `act.end_turn` must be available there",
    );
    Ok(())
}

#[test]
fn the_menu_fixture_samples_no_turn_of_the_players() -> TestResult {
    assert_eq!(
        observed_through(game_app_listening)?.turn_owner(),
        TurnOwner::OtherFaction,
        "with no battle there is no acting faction, let alone the player's",
    );
    Ok(())
}

#[test]
fn a_battle_another_gang_is_acting_in_samples_no_turn_of_the_players() -> TestResult {
    let (app, _port, live) = battle_with_another_gang_acting()?;
    assert_ne!(
        live.active, live.player,
        "the fixture must really split the acting gang from the player's own, or this case \
         cannot tell a present acting gang from the player's: {live:?}",
    );
    assert_eq!(
        observed_in(app)?.turn_owner(),
        TurnOwner::OtherFaction,
        "an acting gang that is not the player's is not the player's turn: {live:?}",
    );
    Ok(())
}
