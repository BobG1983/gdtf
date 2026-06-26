//! Input-bridge overlay seam: highlight, path preview, fire target, reachable-range, and the shared targeting gate.

pub mod fire_target;
pub mod highlight;
pub mod path_preview;
/// The reachable-range DEBUG overlay (GTW-450) — render-only, so the whole module
/// compiles ONLY in a debug build (`#[cfg(debug_assertions)]`, C1). In a release
/// build none of [`ReachableCells`](reachable::ReachableCells) /
/// [`draw_reachable_overlay`](reachable::draw_reachable_overlay) /
/// [`ReachableOverlayEnabled`](reachable::ReachableOverlayEnabled) exists.
#[cfg(debug_assertions)]
pub mod reachable;
pub mod targeting_gate;
