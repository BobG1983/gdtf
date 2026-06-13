use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Init State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Init State");
}
