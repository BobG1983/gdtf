use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Playing State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Playing State");
}
