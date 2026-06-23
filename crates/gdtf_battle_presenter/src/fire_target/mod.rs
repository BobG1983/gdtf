//! The fire-target highlight (GTW-371): the VIEW half of the hover-on-a-fireable-enemy
//! targeting affordance.
//!
//! Per ADR-0001 (the presenter owns ALL sim→view drawing; the sim never reads the
//! presenter) and the `input → presenter → sim` dependency direction, this is split the
//! SAME way as the [`PathPreview`](crate::PathPreview) / [`HighlightRequest`](crate::HighlightRequest)
//! seams:
//!
//! - the PRESENTER (this module) DEFINES the read-seam [`FireTargetHighlight`] resource (the
//!   hovered fireable-enemy CELL + the fire TU cost) and the draw system [`draw_fire_target`]
//!   (the RED tile drawn UNDER the enemy at [`Layer::FireTarget`](crate::Layer) + the OPAQUE
//!   world-space TU-cost label), hard-cut to the active storey, mutated in place;
//! - the INPUT crate (which alone may read `SelectedShooter` + `SelectedFireMode` + the
//!   hovered cell) decides whether the hover is a fireable enemy (the SAME `decide_left_click`
//!   FIRE-rung conditions) and computes the cost via
//!   [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost), then POPULATES this resource (clearing it
//!   off any non-fireable hover) — the [`HighlightRequest`](crate::HighlightRequest) precedent
//!   (the presenter defines the type, input writes it).
//!
//! The selection / fire-mode / hover are NEVER pushed into the sim, the fireable verdict
//! mirrors the FIRE commit's, and the cost EXACTLY equals the `mode_tu_cost` a fire would
//! charge — so the affordance the player sees matches what a click does.

mod draw;

#[cfg(test)]
mod test;

pub use draw::{FireTargetHighlight, FireTargetLabel, FireTargetTile, draw_fire_target};
