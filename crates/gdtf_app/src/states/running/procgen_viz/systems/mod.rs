//! Systems for the DEV-ONLY procgen-visualizer scene (GTW-434) — wiring only.
//!
//! Split by concern: [`build`] (the `OnEnter` model-insert + screen-spawn), [`controls`] (the
//! STEP / AUTO `Changed<Interaction>` readers, C1), [`draw`] (the reveal-sync, C2), and
//! [`lifecycle`] (the `OnExit` model removal). The whole module is
//! `#[cfg(debug_assertions)]`-gated by its parent.

mod build;
mod controls;
mod draw;
mod lifecycle;

pub(in crate::states::running::procgen_viz) use build::{
    insert_viz_model, respawn_quads_on_generate, spawn_viz_screen,
};
pub(in crate::states::running::procgen_viz) use controls::{auto_on_press, step_on_press};
pub(in crate::states::running::procgen_viz) use draw::sync_revealed_quads;
pub(in crate::states::running::procgen_viz) use lifecycle::remove_viz_model;
