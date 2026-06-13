use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Game::BattleScape::BattleRunning State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Game::BattleScape::BattleRunning State");
}
