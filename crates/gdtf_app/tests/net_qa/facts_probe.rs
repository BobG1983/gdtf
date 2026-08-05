use bevy::{
    app::{App, Update},
    ecs::resource::Resource,
    prelude::{Deref, ResMut},
};
use gdtf_app::test_support::{BattleActivity, GameFactsParam};

use super::socket_support::{
    SocketFixture, TestError, TestResult, battle_app_listening, game_app_listening,
};

#[derive(Resource, Deref, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ObservedActivity(Option<BattleActivity>);

fn probe_battle_activity(facts: GameFactsParam, mut observed: ResMut<ObservedActivity>) {
    *observed = ObservedActivity(Some(facts.sample().battle_activity()));
}

fn observed_through(fixture: SocketFixture) -> Result<BattleActivity, TestError> {
    let (mut app, _port) = fixture()?;
    app.init_resource::<ObservedActivity>();
    app.add_systems(Update, probe_battle_activity);
    app.update();
    read_observed(&app)
}

fn read_observed(app: &App) -> Result<BattleActivity, TestError> {
    match **app.world().resource::<ObservedActivity>() {
        Some(activity) => Ok(activity),
        None => Err("the probe system never ran on the real Update schedule".into()),
    }
}

#[test]
fn the_battle_fixture_samples_a_running_battle_through_the_real_facts_param() -> TestResult {
    assert_eq!(
        observed_through(battle_app_listening)?,
        BattleActivity::Running,
        "a fixture resting in BattleRunning must sample as a running battle",
    );
    Ok(())
}

#[test]
fn the_menu_fixture_samples_no_running_battle() -> TestResult {
    assert_eq!(
        observed_through(game_app_listening)?,
        BattleActivity::NotRunning,
        "a fixture resting at the menu has no battle to act in",
    );
    Ok(())
}
