use gdtf_app::test_support::BattleScapeState;
use gdtf_battle_sim::ganger::{Faction, LifeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

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
