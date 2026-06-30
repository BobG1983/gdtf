//! The DEV-ONLY procgen-visualizer's CONFIGURABLE INPUTS (GTW-498) — wiring only.
//!
//! Splits the GTW-498 feature-completeness work (a configurable theme / size / seed / gangs
//! input panel + a Generate action that re-runs procgen) into focused submodules so it never
//! bloats the GTW-434 [`model`](super::model) / [`build`](super::systems) files:
//!
//! - [`resource`] — [`VizConfig`](resource::VizConfig), the typed selected-inputs resource the
//!   panel mutates and Generate reads.
//! - [`components`] — the input-control marker components (theme / size / seed / gang controls,
//!   the Generate button, the size-status text).
//! - [`panel`] — [`spawn_config_panel`](panel::spawn_config_panel), the input-panel builder.
//! - [`apply`] — the selection / commit listeners that mutate the config, the size-validity +
//!   Generate-enable sync, and the Generate action that rebuilds the model.
//!
//! The whole module is `#[cfg(debug_assertions)]`-gated by its parent (`procgen_viz`).

pub(in crate::states::running::procgen_viz) mod apply;
pub(in crate::states::running::procgen_viz) mod components;
pub(in crate::states::running::procgen_viz) mod panel;
pub(in crate::states::running::procgen_viz) mod resource;
