use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Load State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Load State");
}
