//! Bevy state inspection helpers.

use bevy::{
    app::App,
    state::state::{State, States},
};

/// Current value of state type `S`, if registered.
#[must_use]
pub fn current_state<S: States + Clone>(app: &App) -> Option<S> {
    app.world()
        .get_resource::<State<S>>()
        .map(|state| state.get().clone())
}
