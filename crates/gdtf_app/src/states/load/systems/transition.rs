//! `Update` (in `AppState::Load`): leave `Load` once the theme is present.

use bevy::prelude::*;

use crate::states::AppState;

/// Transitions `Load -> Intro` once a [`GdtfTheme`](gdtf_ui::theme::GdtfTheme)
/// has been inserted.
///
/// Runs only when a `GdtfTheme` exists (gated by
/// `run_if(resource_exists::<GdtfTheme>)` in the plugin wiring) and ordered after
/// the poll/resolve system, so the theme is guaranteed present before this queues
/// the state change (bevy-traps rule 3). The `GdtfTheme` is the
/// state-scoped-resource exception that persists past `OnExit(Load)`, so it is
/// available to every later scene.
///
/// It takes `Option<ResMut<NextState<AppState>>>` rather than the bare
/// `ResMut` so it cannot panic if the state machinery is somehow absent
/// (bevy-traps rule 1); under the real wiring `NextState<AppState>` always
/// exists.
pub(in crate::states::load) fn transition_to_intro(next: Option<ResMut<NextState<AppState>>>) {
    if let Some(mut next) = next {
        next.set(AppState::Intro);
    }
}
