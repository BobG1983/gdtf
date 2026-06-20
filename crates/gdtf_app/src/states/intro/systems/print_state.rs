use bevy::prelude::*;

pub(crate) fn print_on_enter() {
    info!("Entered Intro State");
}

pub(crate) fn print_on_exit() {
    info!("Exiting Intro State");
}
