//! Input-bridge overlays: highlight, path preview, fire target, reachable-range, the
//! area-damage-field zone, the cross-level tactical badges, and the shared targeting gate.

/// The GTW-596 cross-level tactical badges — "Signals, Not Scenery" (GTW-593 Option 2):
/// compact corner badges surfacing what the active storey's terrain draw cannot (a
/// fog-VISIBLE enemy above/below, a hole/ledge cell's drop depth, a stair/ladder
/// connector's level-delta). A SHIPPING view (the playability rule), NOT debug-gated.
pub mod cross_level_signals;
/// The GTW-545 area-damage-field overlay — the persistent per-cell VIEW of the sim's live
/// [`FieldRegistry`](gdtf_battle_sim::effects::fields::FieldRegistry) so a seeded field (toxic pool / electrified
/// floor / burning ground) is VISIBLE on the battlefield. A SHIPPING view (the playability rule),
/// so NOT debug-gated, unlike the `reachable` debug overlay.
pub mod field;
pub mod fire_target;
pub mod highlight;
pub mod path_preview;
/// The GTW-568 shared pooled-draw walk — the ONE take-first-N / lazily-grow /
/// hide-surplus loop ([`draw_pool`](pool::draw_pool)) every pooled overlay draw (and the
/// terrain vertical-link draw) reuses, with the `set_if_neq` visibility flips owned by
/// the helper. The message-driven `highlight` overlay is deliberately excluded (retention
/// semantics — see the helper doc).
pub mod pool;
/// The reachable-range DEBUG overlay (GTW-450) — render-only, so the whole module
/// compiles ONLY in a debug build (`#[cfg(debug_assertions)]`, C1). In a release
/// build none of [`ReachableCells`](reachable::ReachableCells) /
/// [`draw_reachable_overlay`](reachable::draw_reachable_overlay) /
/// [`ReachableOverlayEnabled`](reachable::ReachableOverlayEnabled) exists.
#[cfg(debug_assertions)]
pub mod reachable;
pub mod targeting_gate;
