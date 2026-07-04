//! The scene-scaffold helpers (GTW-575): the four stamped system shapes every
//! scene used to hand-write as one-file-per-system, produced by plain helper
//! FNs instead.
//!
//! A new scene's scaffold is now HELPER CALLS in its own `plugin.rs` — same
//! schedules, same `run_if` guards, per-scene plugins keep owning their wiring.
//! Per-scene marker resource TYPES stay per-scene (nested state machines run
//! parent + child scenes simultaneously; one shared generic marker would
//! cross-trigger). A scene that outgrows a shape GRADUATES by dropping the
//! helper call and writing its own system — the helpers never gain mode flags
//! or context parameters (the P9 wrong-abstraction stop rule); the bespoke
//! survivors (`teardown`'s dual-exit `move_on`, `load`'s multi-resource
//! `cleanup` with its persistence exceptions, `battle_running`'s FX-deferred
//! `move_on`, `menu`'s nav-map clear) each carry their own divergence
//! doc-comment.

mod label;
mod systems;

pub(in crate::states) use label::SceneLabel;
pub(in crate::states) use systems::{
    advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
    remove_scoped_resource,
};
