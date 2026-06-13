use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered MainMenu State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting MainMenu State");
}
