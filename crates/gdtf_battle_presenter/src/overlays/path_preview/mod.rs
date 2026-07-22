//! The route path-preview (E7 · GTW-12j): the VIEW half of the move-route preview.
//!
//! Per ADR-0001 (the presenter owns ALL sim→view drawing; the sim never reads the
//! presenter) and the `input → presenter → sim` dependency direction, the preview is
//! split — the SAME split as the [`HighlightRequest`](crate::HighlightRequest) message:
//!
//! - the PRESENTER (this module) DEFINES the read-side [`PathPreview`] resource (the
//!   previewed [`find_path`](gdtf_battle_sim::pathfinder::find_path) route cells + its §48 total cost)
//!   and the draw system [`draw_path_preview`] (the per-step route sprites + the C5
//!   off-storey link-cell marker), hard-cut to the active storey, §53-dimmed on EXPLORED
//!   steps;
//! - the INPUT crate (which alone may read `SelectedShooter` + the new `PathPreviewTarget`)
//!   builds the GTW-353 `PlanningView` from the sim's `SquadVisibility` the SAME way
//!   `dispatch_move` does, calls `find_path`, and POPULATES the resource — the
//!   [`HighlightRequest`](crate::HighlightRequest) precedent (the presenter defines the
//!   type, input writes it).
//!
//! Selection + target are NEVER pushed into the sim, `find_path` is REUSED (no
//! re-implemented routing), and the previewed route + cost EXACTLY match what a commit will
//! accept (the GTW-354 / GTW-355 dependency) — and the route NEVER enters an UNSEEN cell.
//! Consumed by GTW-356 (the two-click flow: click-1 sets `PathPreviewTarget`, click-2
//! commits).

mod draw;
mod resolve;
mod seam;

#[cfg(test)]
mod test;

pub use draw::{PathStepSprite, PathTargetLabel, draw_path_preview};
pub use seam::PathPreview;
