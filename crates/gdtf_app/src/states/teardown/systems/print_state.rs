use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Teardown State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Teardown State");
}
