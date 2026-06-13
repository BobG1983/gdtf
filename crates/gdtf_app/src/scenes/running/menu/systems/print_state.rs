use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Running::Menu State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Running::Menu State");
}
