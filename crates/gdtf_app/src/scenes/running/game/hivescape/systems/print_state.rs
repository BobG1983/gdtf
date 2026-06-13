use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Game::HiveScape State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Game::HiveScape State");
}
