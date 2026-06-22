//! Multi-level path search **route core** (E7 · GTW-12d, ADR-0005 Accepted +
//! `docs/combat/visibility.md` §48): the deterministic weighted search over
//! `(cell, level)` nodes that serves BOTH of GTW-12's deliverables — point-to-point
//! routing ([`find_path`]) and the reachable-range overlay ([`reachable_within`]) —
//! from ONE shared relaxation core.
//!
//! ## What this owns
//!
//! - [`find_path`] — point-to-point A\*: the cheapest legal route from a `start`
//!   to a `goal` [`CellLevel`](crate::metric::CellLevel), or a typed [`PathBlocked`]
//!   when none exists. A\* = Dijkstra + the admissible Chebyshev heuristic.
//! - [`reachable_within`] — the bounded Dijkstra distance-field FLOOD: every
//!   `(cell, level)` reachable within a [`Tu`](crate::ganger::Tu) budget, each with
//!   its cheapest cost. No goal, no heuristic — Dijkstra's native shape.
//! - [`Path`] — the typed route result: the ordered `start..=goal` cells + the total
//!   [`Tu`](crate::ganger::Tu) cost, where the total equals the summed per-step edge
//!   costs along the route (the §48 bit-identity GTW-355 charges).
//! - [`PathBlocked`] — the typed no-route result (never a panic, never an empty path).
//! - [`PathCost`] — the wide accumulated-cost newtype the search relaxes with.
//!
//! ## One core serves both (ADR-0005 OQ-2)
//!
//! [`find_path`] and [`reachable_within`] are thin entry points on a SINGLE
//! priority-queue relaxation ([`core::relax`]) — they differ only by the heuristic
//! (`chebyshev_xy × 4` for routing, `≡ 0` for the flood) and the stop rule (halt at
//! the goal vs prune at the budget). The edge model is the UNION of GTW-350 planar
//! [`pathable_neighbors`](crate::occupancy::pathable_neighbors) (8-connected, octile
//! diagonal) and GTW-351 cross-storey
//! [`traversable_links`](crate::vertical::traversable_links) (flat `link_tu`), so one
//! level-aware search stitches all storeys through the link graph.
//!
//! ## Determinism (C4 — byte-equal replay, NO RNG)
//!
//! A [`BinaryHeap`](std::collections::BinaryHeap) is not stable on equal keys and
//! `HashMap` iteration is nondeterministic, so the frontier orders by the explicit
//! tuple `(total_cost, (z, y, x) cell_key)` — the `(z, y, x)` cell key the FINAL,
//! data-only total order (the GTW-255 `cell_order_key` precedent). Combined with the
//! already-sorted GTW-350/351 edge enumeration, this pins identical routes and
//! identical reachable sets across replays; no raw `HashMap`-order iteration happens
//! in the hot loop.
//!
//! ## Purity (C5)
//!
//! Every function here is a PURE free function (`bevy-traps.md` #7) over the borrowed
//! [`OccupancyGrid`](crate::occupancy::OccupancyGrid) /
//! [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) /
//! [`CombatTuning`](crate::tuning::CombatTuning) snapshot — no `&mut World`, no
//! system, no RNG. The search reads the change-driven grids as a snapshot; it NEVER
//! rebuilds them per query (resolution.md). It plans and totals route cost; it never
//! charges TU — the per-step writers charge at commit (§48).
//!
//! Scope: the search core ONLY — no tile work, no movement dispatch. Consumed by
//! GTW-354 (constrain dispatch), GTW-357 (range overlay), GTW-358 (path preview),
//! GTW-353 (visibility-gated planning).

mod core;
mod path;
mod search;

#[cfg(test)]
mod test;

pub use path::{Path, PathBlocked, PathCost};
pub use search::{find_path, reachable_within};
