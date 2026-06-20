use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Game::Setup State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Game::Setup State");
}
