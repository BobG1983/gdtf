//! The reachable-range move overlay (E7 · GTW-12i): the VIEW half of the move-range
//! preview.
//!
//! Per ADR-0001 (the presenter owns ALL sim→view drawing; the sim never reads the
//! presenter) and the `input → presenter → sim` dependency direction, the overlay is
//! split:
//!
//! - the PRESENTER (this module) DEFINES the read-seam [`ReachableOverlay`] resource (the
//!   `Vec<(CellLevel, Tu)>` the SELECTED ganger can reach) and the two draw systems — the
//!   per-cell tint [`draw_reachable_overlay`] + the per-cell TU-cost label
//!   [`draw_reachable_labels`] — both hard-cut to the active storey;
//! - the INPUT crate (which alone may read `SelectedShooter` + the selected ganger's
//!   `Position` / `Tu`) builds the GTW-353 `PlanningView` from the sim's `SquadVisibility`
//!   the SAME way `dispatch_move` does, calls `reachable_within`, and POPULATES the resource
//!   — the [`HighlightRequest`](crate::HighlightRequest) precedent (the presenter defines the
//!   type, input writes it).
//!
//! Selection is NEVER pushed into the sim, `reachable_within` is REUSED (no re-implemented
//! reachability), and the lit set EXACTLY matches the cells a commit will accept (the GTW-354
//! constrained `dispatch_move` dependency). Consumed by GTW-358 (path preview) + the two-click
//! flow (GTW-356).

mod overlay;

#[cfg(test)]
mod test;

pub use overlay::{
    LabelPool, ReachableLabel, ReachableOverlay, ReachableTint, draw_reachable_labels,
    draw_reachable_overlay,
};
