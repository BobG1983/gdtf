//! Input-bridge overlay seam: highlight, path preview, fire target, reachable-range, the
//! area-damage-field zone, and the shared targeting gate.

/// The GTW-545 area-damage-field overlay — the persistent per-cell VIEW of the sim's live
/// [`FieldRegistry`](gdtf_battle_sim::FieldRegistry) so a seeded field (toxic pool / electrified
/// floor / burning ground) is VISIBLE on the battlefield. A SHIPPING view (the playability rule),
/// so NOT debug-gated, unlike the `reachable` debug overlay.
pub mod field;
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
