//! The generic state read-back accessor (GTW-576) — the shared half of the
//! per-file `app_state` / `running_state` helper copies.

use bevy::{
    app::App,
    state::state::{State, States},
};

/// The current value of state `S`, or `None` if `S` was never registered on the
/// app — so a suite asserts `current_state::<RunningState>(&app) == Some(..)`
/// without hand-rolling the `resource::<State<S>>().get().clone()` chain.
#[must_use]
pub fn current_state<S: States + Clone>(app: &App) -> Option<S> {
    app.world()
        .get_resource::<State<S>>()
        .map(|state| state.get().clone())
}
