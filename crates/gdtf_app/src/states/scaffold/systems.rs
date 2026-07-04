//! The four stamped system SHAPES as value-capturing helper fns (GTW-575).
//!
//! Each helper is a plain fn returning an ordinary `Commands`/`ResMut`-param
//! system (never `&mut World` — bevy-traps #7): the scene plugin calls it and
//! registers the returned system in the schedule + `run_if` guard it owns, so
//! registration semantics are byte-for-byte what the deleted one-file systems
//! had. Compile-time closures/generics throughout — no runtime descriptor
//! tables (P4).

use bevy::{prelude::*, state::state::FreelyMutableState};

use super::SceneLabel;

/// The enter-side scene logger: `info!("Entered <label> State")`, exactly the
/// text the scene's old `print_state.rs::print_on_enter` printed.
///
/// Registered `OnEnter(<scene state>)` by the owning scene plugin.
pub(in crate::states) fn log_scene_enter(label: SceneLabel) -> impl Fn() {
    let message = format!("Entered {} State", *label);
    move || info!("{message}")
}

/// The exit-side scene logger: `info!("Exiting <label> State")`, exactly the
/// text the scene's old `print_state.rs::print_on_exit` printed.
///
/// Registered `OnExit(<scene state>)` by the owning scene plugin.
pub(in crate::states) fn log_scene_exit(label: SceneLabel) -> impl Fn() {
    let message = format!("Exiting {} State", *label);
    move || info!("{message}")
}

/// The completion-marker insert: `commands.insert_resource(M::default())`, the
/// old one-line `track_complete.rs` system.
///
/// Generic over the scene's OWN marker type — every scene keeps its own marker
/// `Resource` (a shared marker would cross-trigger the parent and child scenes
/// a nested state machine runs simultaneously). The owning plugin registers it
/// gated `in_state(<scene>) && not(resource_exists::<M>)`, so the insert fires
/// once per scene span.
pub(in crate::states) fn insert_completion_marker<M: Resource + Default>() -> impl Fn(Commands) {
    |mut commands: Commands| commands.insert_resource(M::default())
}

/// The scoped-resource remove: `commands.remove_resource::<R>()`, the old
/// one-line `cleanup.rs` system.
///
/// Registered `OnExit(<scene state>)` by the owning plugin so the per-span
/// resource never leaks into a re-entry (bevy-traps #1 — resources are not
/// state-scoped by the engine). `remove_resource` is a no-op when the resource
/// is absent, so removing a maybe-never-inserted resource is always safe.
pub(in crate::states) fn remove_scoped_resource<R: Resource>() -> impl Fn(Commands) {
    |mut commands: Commands| commands.remove_resource::<R>()
}

/// The move-on transition: `NextState::set(<target>)`, the old one-line
/// `move_on.rs` system, capturing the target state VALUE.
///
/// The owning plugin registers it gated
/// `in_state(<scene>) && resource_exists::<M>` (its scene's completion
/// marker), so the scene advances exactly when its marker appears.
pub(in crate::states) fn advance_state_to<S: FreelyMutableState>(
    target: S,
) -> impl Fn(ResMut<NextState<S>>) {
    move |mut next: ResMut<NextState<S>>| next.set(target.clone())
}
