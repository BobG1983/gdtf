//! Pathfinder route-core tests (E7 · GTW-12d, C6) — RELATIONS-ONLY, no pinned
//! shipped tunable magnitudes (the brittle-test rule): every cost is asserted by its
//! DERIVATION over the tuning the fixtures expose, never a default magnitude.
//!
//! - [`same_storey`] — a same-storey route avoids a wall and reaches the goal.
//! - [`cross_storey`] — a route THROUGH a vertical link, with the `link_tu`
//!   accumulating across the hop (the §48 cost-accumulation check).
//! - [`blocked`] — an unreachable goal yields [`PathBlocked`](super::PathBlocked).
//! - [`bit_identity`] — the [`Path`](super::Path) total EQUALS the summed per-step
//!   edge costs along the route (the §48 bit-identity).
//! - [`determinism`] — same inputs → byte-identical [`Path`](super::Path) AND
//!   byte-identical [`reachable_within`](super::reachable_within) set, asserted twice.
//! - [`budget`] — [`reachable_within`](super::reachable_within) respects the budget
//!   (a cell just over budget excluded, one within included).
//! - [`visibility_gated`] — GTW-353: UNSEEN cells are non-routable (route around them /
//!   excluded from the flood), EXPLORED stays routable, and the visibility-aware
//!   blocking predicate (own-squad always, enemy iff VISIBLE, scatter/walls always).
//! - [`tag_blocking`] — GTW-501 (C6 a/b/c): the pathfinder reads the TAG-derived
//!   path-blocking surface, not [`TerrainKind`](crate::occupancy::TerrainKind) — a tagged
//!   cell blocks, the same cell untagged does not, and a kind-default `Wall` still blocks
//!   (zero regression).

mod bit_identity;
mod blocked;
mod budget;
mod cross_storey;
mod determinism;
mod same_storey;
mod support;
mod tag_blocking;
mod visibility_gated;
