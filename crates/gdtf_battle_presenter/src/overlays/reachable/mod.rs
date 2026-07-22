//! The reachable-range overlay (GTW-387 C3): the presenter-owned read-side resource +
//! draw system.
//!
//! Per ADR-0001 (the presenter owns ALL sim→view drawing) and the
//! `input → presenter → sim` dependency direction, the overlay is split — the SAME
//! split as the [`PathPreview`](crate::PathPreview) resource:
//!
//! - the PRESENTER (this module) DEFINES the read-side
//!   [`ReachableCells`](crate::ReachableCells) resource
//!   (the cells the selected ganger can reach within its remaining TU, each with its
//!   cheapest accumulated cost) and the draw system
//!   [`draw_reachable_overlay`](crate::draw_reachable_overlay) (the
//!   per-cell hard-cut-to-active-storey sprites);
//! - the INPUT crate calls [`reachable_within`](gdtf_battle_sim::pathfinder::reachable_within) for
//!   the selected ganger and POPULATES the resource — the
//!   [`PathPreview`](crate::PathPreview) precedent (the presenter defines the type,
//!   input writes it).
//!
//! Selection and [`ActiveLevel`](crate::ActiveLevel) are NEVER pushed into the sim.
//! The overlay follows `PageUp` with no extra wiring — the draw system reads
//! `Res<ActiveLevel>` live, so after a level switch only the new active storey's cells
//! render.

mod overlay;

#[cfg(test)]
mod test;

pub use overlay::{
    REACHABLE_OVERLAY_ENV, ReachableCellSprite, ReachableCells, ReachableOverlayEnabled,
    draw_reachable_overlay,
};
