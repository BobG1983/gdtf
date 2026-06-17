//! Multi-frame driving helper for headless test apps.

use bevy::app::App;

/// Drives `app` forward up to `max_updates` times, stopping early when
/// `predicate` holds.
///
/// Calls [`App::update`] and then evaluates `predicate(&app)` after each update,
/// returning `true` as soon as the predicate is satisfied. Returns `false` if the
/// predicate never holds within `max_updates` updates. Useful for waiting on a
/// multi-frame state transition without hard-coding a frame count.
pub fn advance_until(app: &mut App, predicate: impl Fn(&App) -> bool, max_updates: u32) -> bool {
    for _ in 0..max_updates {
        app.update();
        if predicate(app) {
            return true;
        }
    }
    false
}
