use cobalt_test_utils::advance_until;
use gdtf_battle_sim::ganger::{Faction, LifeState};
use gdtf_game::test_support::BattleScapeState;

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
    let mut app = driven_battle_app();

    down_faction(&mut app, TARGET_FACTION, LifeState::Dead);

    advance_until(&mut app, left_battle_running);
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a census-emitted BattleWon must advance BattleRunning → AnimateOut end-to-end",
    );
}

#[test]
fn census_loss_ends_the_battle_to_animate_out() {
    let mut app = driven_battle_app();

    down_faction(&mut app, SHOOTER_FACTION, LifeState::Dead);

    advance_until(&mut app, left_battle_running);
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a census-emitted BattleLost must advance BattleRunning → AnimateOut end-to-end",
    );
}
