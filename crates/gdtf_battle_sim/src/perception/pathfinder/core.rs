//! The single weighted-search **relaxation core** (E7 · GTW-12d, ADR-0005 "one core
//! serves both"): the priority-queue Dijkstra/A\* loop that both
//! [`find_path`](super::find_path) and [`reachable_within`](super::reachable_within)
//! are thin entry points on.
//!
//! It is PURE (`bevy-traps.md` #7): free functions over the borrowed
//! [`OccupancyGrid`] / [`VerticalLinkGraph`] / [`CombatTuning`] snapshot — no
//! `&mut World`, no system, no RNG. The two entry points differ ONLY by the
//! heuristic and the stop rule fed in here; the relaxation, the edge enumeration,
//! and the deterministic tie-break are shared (ADR-0005 OQ-2: one priority queue,
//! one relaxation, one tie-break to test and maintain).
//!
//! Determinism (C4): a [`BinaryHeap`] is NOT stable on equal keys and `HashMap`
//! iteration is nondeterministic, so the frontier orders by the explicit
//! [`FrontierNode`] tuple `(total_cost, (z, y, x) cell_key)` — the `(z, y, x)` cell
//! key the FINAL, data-only total order (the GTW-255 `cell_order_key` precedent).
//! The edge enumeration is already pre-sorted by GTW-350
//! [`pathable_neighbors`](crate::occupancy::pathable_neighbors) (the `(z, y, x)`
//! offset table) then GTW-351
//! [`traversable_links`](crate::vertical::traversable_links) (graph-index order),
//! and the cell-key tie-break makes any equal-cost frontier choice independent of
//! heap internals / insertion timing — so two replays expand identically.

use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

use bevy::{platform::collections::HashMap, prelude::Entity};

use super::{path::PathCost, planning::PlanningView};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::CellLevel,
    occupancy::{OccupancyGrid, pathable_neighbors},
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    vertical::{VerticalLinkGraph, traversable_links},
    visibility::FactionRelation,
};

/// The `(z, y, x)` total-ordering key of a [`CellLevel`] — the FINAL, data-only
/// tie-break component of the frontier order (C4).
///
/// A `(i32, i32, i32)` tuple — the framework carve-out for an ordering key over the
/// already-typed [`CellLevel`] coordinates (a sort key is index plumbing, not a
/// fresh domain scalar; `no-bare-types.md` rule 4). Ordered `(z = storey, then
/// y = row, then x = column)`, the SAME total order
/// [`auto_select`](https://linear.app/robert-gardner/issue/GTW-255)'s `cell_order_key`
/// and GTW-350's planar-offset table use — reproducible across runs, NOT the
/// nondeterministic `HashMap`/heap iteration order.
type CellKey = (i32, i32, i32);

/// The `(z, y, x)` cell key of a `(cell, level)` — reads the coordinates through
/// [`CellLevel`]'s `Deref` to its `IVec3` and orders them storey-major.
fn cell_key(cell: CellLevel) -> CellKey {
    // `cell` derefs `IVec3`; `.z` is the storey index, `.y` the row, `.x` the column.
    (cell.z, cell.y, cell.x)
}

/// One entry on the search frontier — a `(cell, level)` reached at an accumulated
/// cost, ordered for a deterministic MIN-priority queue.
///
/// The [`Ord`] is the C4 tie-break tuple: `(total_cost, (z, y, x) cell_key)` —
/// LOWER cost first, and on EQUAL cost the LOWER `(z, y, x)` cell key first (the
/// data-only final order). For point-to-point A\* the `priority` is `cost + h`
/// (the heuristic-focused order) while the cell key still breaks ties; for the
/// goal-free flood `priority == cost` (`h ≡ 0`). [`BinaryHeap`] is a MAX-heap, so
/// the search wraps this in [`Reverse`] to pop the smallest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrontierNode {
    /// The ordering priority: the accumulated [`PathCost`] PLUS the heuristic
    /// estimate to the goal (`cost + h`; `h ≡ 0` for the flood). This is what the
    /// heap orders by FIRST — A\* explores low-`f` nodes first, Dijkstra (`h ≡ 0`)
    /// explores low-cost nodes first.
    priority: PathCost,
    /// The accumulated cost to REACH this node (the real route cost so far, no
    /// heuristic) — used to relax neighbours and to record the settled distance.
    cost:     PathCost,
    /// The `(cell, level)` this frontier entry reaches.
    cell:     CellLevel,
}

impl Ord for FrontierNode {
    /// Order by `(priority, (z, y, x) cell_key)` — the C4 deterministic tie-break.
    /// Cost-then-cell-key is a TOTAL order, so equal-priority frontier entries
    /// resolve by the data-only cell key, never by heap internals.
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| cell_key(self.cell).cmp(&cell_key(other.cell)))
    }
}

impl PartialOrd for FrontierNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The borrowed grids + tuning + visibility-aware planning gate the search reads as a
/// change-driven SNAPSHOT (C5) — bundled so the relaxation core and both entry points
/// thread ONE param.
///
/// A read-only view: the [`OccupancyGrid`] (walkable/blocked terrain), the
/// [`VerticalLinkGraph`] (cross-storey links), the [`CombatTuning`] (the flat
/// [`LinkTu`](crate::tuning::LinkTu)), the [`FloorCostGrid`] (per-cell floor move
/// costs, GTW-396 Decision C1 — replaces `tuning.move_costs` in the planar edge
/// path), and the GTW-353 [`PlanningView`] (the squad fog with the occupant-faction
/// resolver that gates which candidates are routable). The search NEVER rebuilds
/// these per query — it reads the already-maintained resources (ADR-0005 /
/// resolution.md). It is framework plumbing (a borrow bundle), not a domain value.
/// Generic over the occupant→[`FactionRelation`] resolver `R` the planning view carries.
pub(super) struct SearchGrids<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    /// The authoritative walkable/blocked terrain surface.
    pub(super) grid:        &'a OccupancyGrid,
    /// The authored cross-storey stair/ladder links.
    pub(super) links:       &'a VerticalLinkGraph,
    /// The balance tuning — the flat link cost + other combat coefficients.
    /// NOTE: `move_costs` is no longer read from here for floor steps (GTW-396
    /// Decision C1); `floor_costs` is the sole step-cost source.
    pub(super) tuning:      &'a CombatTuning,
    /// The per-cell floor move-cost surface (GTW-396) — read by `pathable_neighbors`
    /// for every planar destination cell instead of `tuning.move_costs`.
    pub(super) floor_costs: &'a FloorCostGrid,
    /// The MOVER'S movement-cost factor (GTW-444) — the per-ganger "Hampered" slowdown,
    /// read from the mover's [`InflictedInjuries`](crate::injuries::InflictedInjuries)
    /// ledger by the caller and applied to EVERY planar step cost (the preview side of the
    /// preview==charge identity, C3). [`MovementCostFactor::IDENTITY`] for an uninjured
    /// mover (no scaling).
    pub(super) factor:      MovementCostFactor,
    /// The visibility-aware planning gate — the squad fog + occupant-faction resolver
    /// deciding which candidate `(cell, level)`s are routable (C1 / C2).
    pub(super) planning:    &'a PlanningView<'a, R>,
}

impl<R> SearchGrids<'_, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    /// Enumerate the outgoing edges of `origin` — the UNION of GTW-350 planar
    /// [`pathable_neighbors`] (8-connected, octile-priced via [`FloorCostGrid`]) and
    /// GTW-351 cross-storey [`traversable_links`] (flat `link_tu`), each as a
    /// `(neighbour, step_cost)` pair, FILTERED to the visibility-routable candidates
    /// (C1 / C2).
    ///
    /// Both halves are ALREADY emitted in a fixed, deterministic order (the planar
    /// `(z, y, x)` offset table, then the graph-index link order), so the relaxation
    /// never iterates a raw `HashMap` in the hot loop (C4). Planar edges are listed
    /// first, then vertical hops — a fixed concatenation. The GTW-353 visibility gate
    /// then drops any neighbour that is non-routable: UNSEEN cells (C1) and cells
    /// blocked within-routable by geometry or a blocking occupant (C2). The gate only
    /// REMOVES edges in a deterministic, set-membership way — it never reorders the
    /// survivors or alters their octile / link costs, so the C4 determinism + the cost
    /// model are untouched.
    ///
    /// # GTW-387: per-half routing predicates
    ///
    /// The two halves apply DIFFERENT routability predicates:
    ///
    /// - **Planar** (`pathable_neighbors`) — the FULL gate
    ///   ([`PlanningView::is_routable`]: C1 UNSEEN + C2). An ordinary 8-connected step
    ///   to an UNSEEN floor tile stays non-routable: no fog relaxation, no
    ///   see-through-walls hole.
    /// - **Vertical** (`traversable_links`) — the C1-RELAXED gate
    ///   ([`PlanningView::is_routable_link`]: C2 only). Every element is a
    ///   setup-validated [`VerticalLinkGraph`] far endpoint — the graph is the
    ///   "known-link" oracle, so planning onto an as-yet-unseen storey via a known
    ///   stair or ladder is permitted. C2 (geometry + visible occupant) still applies
    ///   unconditionally; only the C1 fog-explored check is lifted for link endpoints.
    ///
    /// The ELEMENT ORDER of survivors is identical to the previous single-filter form
    /// (planar order preserved, vertical order preserved, planar-before-vertical
    /// preserved), so C4 determinism is untouched.
    fn edges(&self, origin: CellLevel) -> Vec<(CellLevel, Tu)> {
        // Planar steps keep the FULL gate (C1 UNSEEN + C2). An UNSEEN floor tile
        // reached by an ordinary step stays non-routable — no fog relaxation.
        let planar = pathable_neighbors(origin, self.grid, self.floor_costs, self.factor)
            .filter(|(neighbour, _)| *self.planning.is_routable(*neighbour, self.grid));
        // GTW-387: a vertical hop's far endpoint comes from the VALIDATED
        // VerticalLinkGraph (links_from(origin)) — it IS a known link. Relax C1 for
        // it (you may plan onto an UNSEEN storey VIA a known link) but KEEP C2: the
        // landing cell must still be geometrically open and unblocked by a visible
        // occupant. is_routable_link skips ONLY the explored check.
        let vertical = traversable_links(origin, self.links, self.tuning.link_tu)
            .filter(|(neighbour, _)| *self.planning.is_routable_link(*neighbour, self.grid));
        planar.chain(vertical).collect()
    }
}

/// The settled distance field — the cheapest accumulated [`PathCost`] reached for
/// each settled `(cell, level)`, plus the predecessor for route reconstruction.
///
/// Keyed by `(cell, level)`, NOT iterated in the hot loop (only point-LOOKED-UP for
/// the relax check and walked backwards once for reconstruction), so its
/// nondeterministic `HashMap` order never leaks into the route or the reachable set
/// (C4). The predecessor is `None` for the start cell.
pub(super) struct DistanceField {
    /// The cheapest cost settled for each reached `(cell, level)`.
    cost: HashMap<CellLevel, PathCost>,
    /// The predecessor of each reached `(cell, level)` on its cheapest route — the
    /// back-pointers [`reconstruct`](DistanceField::reconstruct) walks. `None` for
    /// the start cell.
    prev: HashMap<CellLevel, Option<CellLevel>>,
}

impl DistanceField {
    /// The settled cost of `cell`, if it was reached within the search.
    #[must_use]
    pub(super) fn cost_of(&self, cell: &CellLevel) -> Option<PathCost> {
        self.cost.get(cell).copied()
    }

    /// Every settled `(cell, level)` paired with its cheapest accumulated cost — the
    /// reachable distance field [`reachable_within`](super::reachable_within) yields.
    ///
    /// Sorted by the `(z, y, x)` cell key so the returned collection is DETERMINISTIC
    /// (C4) — the underlying `HashMap`'s iteration order never escapes.
    #[must_use]
    pub(super) fn settled_sorted(&self) -> Vec<(CellLevel, PathCost)> {
        let mut out: Vec<(CellLevel, PathCost)> = self
            .cost
            .iter()
            .map(|(&cell, &cost)| (cell, cost))
            .collect();
        out.sort_by_key(|(cell, _)| cell_key(*cell));
        out
    }

    /// The per-step ENTRY costs along the reconstructed `route` (the `start..=goal`
    /// cell list), aligned to `route[1..]` — `step_costs[i]` is the [`Tu`] to enter
    /// `route[i + 1]` from `route[i]`.
    ///
    /// Each step cost is the SETTLED accumulated-cost DELTA the relaxation already
    /// recorded — `cost(route[i + 1]) − cost(route[i])` — NOT a re-run of the octile /
    /// link cost math. Because the route is the cheapest-cost reconstruction, those
    /// deltas are exactly the per-edge [`PathCost::add_step`](super::path::PathCost::add_step)
    /// increments that built the goal's total, so their sum is the route total
    /// bit-for-bit (the §48 bit-identity). A single in-domain step is a `u8`
    /// [`Tu`](crate::ganger::Tu) (octile `≤ round(255 × √2)` is bounded; the search
    /// only relaxes edges priced from a `u8` `move_cost` / `link_tu`), so the per-step
    /// delta narrows to [`Tu`] exactly via [`PathCost::to_tu`](super::path::PathCost::to_tu).
    /// A cell whose cost was somehow not settled (impossible for a reconstructed route)
    /// contributes [`PathCost::ZERO`].
    #[must_use]
    pub(super) fn step_costs(&self, route: &[CellLevel]) -> Vec<Tu> {
        route
            .windows(2)
            .map(|window| {
                let from = self.cost_of(&window[0]).unwrap_or(PathCost::ZERO);
                let to = self.cost_of(&window[1]).unwrap_or(PathCost::ZERO);
                // The cheapest route is monotone in cost (non-negative edges), so
                // `to >= from`; saturating to ZERO on the impossible reverse keeps it
                // panic-free without masking a real bug (a reconstructed route never
                // hits it).
                PathCost::new((*to).saturating_sub(*from)).to_tu()
            })
            .collect()
    }

    /// Reconstruct the ordered `start..=goal` route to `goal` by walking the
    /// predecessor back-pointers, or `None` if `goal` was never reached.
    ///
    /// Walks `prev` from `goal` back to the start (whose predecessor is `None`),
    /// then reverses — producing the route in step order (`start` first, `goal`
    /// last). A predecessor chain is acyclic by Dijkstra construction (each node is
    /// settled once at its cheapest cost), so the walk terminates.
    #[must_use]
    pub(super) fn reconstruct(&self, goal: CellLevel) -> Option<Vec<CellLevel>> {
        // `goal` must have been reached for a route to exist.
        self.prev.get(&goal)?;
        let mut route = vec![goal];
        let mut current = goal;
        while let Some(Some(predecessor)) = self.prev.get(&current).copied() {
            route.push(predecessor);
            current = predecessor;
        }
        route.reverse();
        Some(route)
    }
}

/// The shared Dijkstra/A\* **relaxation** — expand the frontier from `start`,
/// ordered by the deterministic `(cost + h, (z, y, x) cell_key)` tuple, until the
/// `stop` rule says to halt, settling the cheapest cost (and predecessor) of every
/// reached `(cell, level)` into a [`DistanceField`] (ADR-0005's one core).
///
/// The single relaxation loop both entry points share. Two closures specialise it:
///
/// - `heuristic(cell) -> PathCost` — the admissible estimate from `cell` to the goal
///   added to the popped cost to form the priority. For
///   [`reachable_within`](super::reachable_within) it is `≡ 0` (Dijkstra's native
///   flood shape); for [`find_path`](super::find_path) it is `chebyshev_xy × 4` (A\*).
/// - `stop(cell, cost) -> StopRule` — inspected when a node is POPPED (settled):
///   [`StopRule::Done`] halts the search (the goal was reached, for
///   [`find_path`](super::find_path)); [`StopRule::Prune`] settles the node but does
///   NOT expand its neighbours (the budget boundary, for
///   [`reachable_within`](super::reachable_within)); [`StopRule::Expand`] settles AND
///   relaxes its neighbours (the normal case).
///
/// A node is settled the first time it is POPPED (its cheapest cost — Dijkstra's
/// invariant on non-negative edges, which all `move_cost`/`link_tu` are); later,
/// stale heap entries for an already-settled cell are skipped. The frontier is
/// seeded with `start` at [`PathCost::ZERO`] and predecessor `None`.
pub(super) fn relax<H, S, R>(
    start: CellLevel,
    grids: SearchGrids<'_, R>,
    heuristic: H,
    stop: S,
) -> DistanceField
where
    H: Fn(CellLevel) -> PathCost,
    S: Fn(CellLevel, PathCost) -> StopRule,
    R: Fn(Entity) -> FactionRelation,
{
    let mut field = DistanceField {
        cost: HashMap::default(),
        prev: HashMap::default(),
    };
    let mut frontier: BinaryHeap<Reverse<FrontierNode>> = BinaryHeap::new();

    // Seed the start node at cost 0, predecessor None.
    field.cost.insert(start, PathCost::ZERO);
    field.prev.insert(start, None);
    frontier.push(Reverse(FrontierNode {
        priority: heuristic(start),
        cost:     PathCost::ZERO,
        cell:     start,
    }));

    while let Some(Reverse(node)) = frontier.pop() {
        // Skip a STALE entry: this cell was already settled at a cheaper cost via a
        // later-relaxed-but-earlier-popped route. (Dijkstra settles on first pop.)
        if field.cost.get(&node.cell).copied() != Some(node.cost) {
            continue;
        }

        match stop(node.cell, node.cost) {
            // Goal reached (find_path): the route to `node.cell` is final — halt.
            StopRule::Done => break,
            // Budget boundary (reachable_within): keep this cell in the field but do
            // NOT pay to expand beyond it.
            StopRule::Prune => continue,
            // Normal expansion: relax every outgoing edge.
            StopRule::Expand => {}
        }

        for (neighbour, step) in grids.edges(node.cell) {
            let next_cost = node.cost.add_step(step);
            // Relax: record `neighbour` iff this route reaches it more cheaply than
            // any seen so far (or it is unseen). A strict `<` keeps the FIRST
            // cheapest predecessor; an equal-cost alternative does NOT overwrite it,
            // which (with the deterministic pop order) pins the reconstructed route.
            let improved = field
                .cost
                .get(&neighbour)
                .is_none_or(|existing| next_cost < *existing);
            if !improved {
                continue;
            }
            field.cost.insert(neighbour, next_cost);
            field.prev.insert(neighbour, Some(node.cell));
            frontier.push(Reverse(FrontierNode {
                priority: PathCost::new(*next_cost + *heuristic(neighbour)),
                cost:     next_cost,
                cell:     neighbour,
            }));
        }
    }

    field
}

/// What the [`relax`] loop does with a node when it is POPPED (settled) — the
/// per-entry-point specialisation of the shared core.
///
/// A named enum (no-bare-types) so the stop rule reads as intent, not a bare
/// bool/`Option`. [`find_path`](super::find_path) returns [`Done`](StopRule::Done)
/// at the goal; [`reachable_within`](super::reachable_within) returns
/// [`Prune`](StopRule::Prune) once a node's cost exceeds the budget and
/// [`Expand`](StopRule::Expand) otherwise; a plain full Dijkstra returns
/// [`Expand`](StopRule::Expand) everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopRule {
    /// Settle this node, then HALT the whole search (the goal was reached).
    Done,
    /// Settle this node but do NOT expand its neighbours (a boundary node).
    Prune,
    /// Settle this node AND relax its neighbours (the normal case).
    Expand,
}
