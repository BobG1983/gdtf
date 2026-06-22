//! The two route-search **entry points** (E7 · GTW-12d): [`find_path`]
//! (point-to-point A\*) and [`reachable_within`] (the bounded distance-field
//! flood), both thin wrappers over the ONE shared relaxation core ([`super::core`],
//! ADR-0005 "one core serves both").
//!
//! Both are PURE free functions (`bevy-traps.md` #7) over the borrowed grids +
//! tuning snapshot — no `&mut World`, no system, no RNG. They differ ONLY in the
//! heuristic and the stop rule they hand the core: `find_path` adds an admissible
//! A\* heuristic and halts at the goal; `reachable_within` uses `h ≡ 0` and prunes
//! at the TU budget.

use super::{
    core::{SearchGrids, StopRule, relax},
    path::{Path, PathBlocked, PathCost},
};
use crate::{
    ganger::Tu, metric::CellLevel, occupancy::OccupancyGrid, tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};

/// The minimum positive per-step move cost on the grid — `4`, the cheapest
/// [`MoveCost`](crate::tuning::MoveCost) (`open`).
///
/// Load-bearing for the A\* heuristic's admissibility (ADR-0005 OQ-2 / §"Context":
/// "The minimum positive move cost on the grid is 4"): scaling the planar
/// step-distance by the MINIMUM per-step cost guarantees the heuristic never
/// overestimates the true cheapest route, which is what keeps A\* admissible (and so
/// optimal). Documented as a constant rather than read from tuning because it is the
/// FLOOR of the cost scale — the heuristic must bound from below by the cheapest
/// possible step, never the actual (possibly dearer) authored `open` cost; using the
/// hard floor stays admissible even if `open` is tuned upward.
const MIN_MOVE_COST: u32 = 4;

/// The admissible A\* heuristic from `from` to `goal` — the planar Chebyshev
/// step-distance times the minimum per-step cost (ADR-0005 OQ-2).
///
/// `h = chebyshev_xy(from, goal) × `[`MIN_MOVE_COST`]. The Chebyshev distance
/// (`max(|Δx|, |Δy|)`) is the fewest 8-connected planar STEPS between the two cells;
/// times the cheapest possible per-step cost it is a lower bound on the true route
/// cost, so `h` NEVER overestimates — the admissibility condition.
///
/// **Admissibility across storeys.** The heuristic ignores z and link cost entirely:
/// it contributes `0` for a pure storey change. Since the true cost of any storey
/// change is `> 0` (a positive `link_tu`), ignoring it can only UNDER-estimate, never
/// over-estimate — so `h` stays admissible across storeys (it merely weakens toward
/// Dijkstra when the goal is mostly above/below, which is safe, just slower). A
/// tighter `+ |Δz| × min_link_cost` term is a future speed optimization, not a
/// correctness requirement.
fn chebyshev_heuristic(from: CellLevel, goal: CellLevel) -> PathCost {
    // `from`/`goal` deref `IVec3`; the planar Chebyshev distance ignores z (the
    // storey), which the cross-storey-admissibility argument above relies on.
    let dx = (from.x - goal.x).unsigned_abs();
    let dy = (from.y - goal.y).unsigned_abs();
    let steps = dx.max(dy);
    PathCost::new(steps * MIN_MOVE_COST)
}

/// Find the cheapest legal route from `start` to `goal` over the occupancy grid +
/// vertical-link graph, or [`PathBlocked`] if no route exists (C1).
///
/// **A\* = Dijkstra + an admissible heuristic** on the ONE shared relaxation core
/// ([`relax`]): it expands the frontier ordered by `(cost + h, (z, y, x) cell_key)`
/// — the heuristic [`chebyshev_heuristic`] focusing the search toward `goal`, the
/// `(z, y, x)` cell key breaking ties deterministically (C4) — and halts the moment
/// `goal` is popped ([`StopRule::Done`]). Because `h` is admissible (never
/// overestimates — see [`chebyshev_heuristic`]), the first time `goal` settles it is
/// at its cheapest cost, so the route is optimal.
///
/// Edges are the UNION of GTW-350 planar [`pathable_neighbors`](crate::occupancy::pathable_neighbors)
/// (8-connected, octile diagonal) and GTW-351 cross-storey
/// [`traversable_links`](crate::vertical::traversable_links) (flat `link_tu`) — the
/// route stitches storeys solely through the link graph (ADR-0005 "Vertical
/// stitching"). The cost model is read from `tuning` (the per-terrain
/// [`MoveCosts`](crate::tuning::MoveCosts) table + the flat
/// [`LinkTu`](crate::tuning::LinkTu)); the search NEVER charges TU — it plans and
/// totals, and the per-step writers charge at commit (§48).
///
/// The returned [`Path`] runs `start..=goal` in step order with a
/// [`total`](Path::total) that equals the sum of the per-step edge costs along it
/// (the §48 bit-identity). The degenerate `start == goal` is the one-cell route
/// `[start]` at total `0`.
///
/// PURE (`bevy-traps.md` #7): a free function over the borrowed snapshot — no
/// `&mut World`, no system, no RNG, and it never rebuilds the grids per query (C5).
///
/// # Errors
///
/// Returns [`PathBlocked`] when `goal` is not reachable from `start` over the
/// walkable grid + links (a typed no-route result — never a panic, never an empty
/// [`Path`]).
pub fn find_path(
    start: CellLevel,
    goal: CellLevel,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
) -> Result<Path, PathBlocked> {
    let grids = SearchGrids {
        grid,
        links,
        tuning,
    };
    let field = relax(
        start,
        grids,
        |cell| chebyshev_heuristic(cell, goal),
        // Halt the moment the goal settles; expand every other popped node.
        |cell, _cost| {
            if cell == goal {
                StopRule::Done
            } else {
                StopRule::Expand
            }
        },
    );

    // Reconstruct the route; absence means `goal` was never reached → blocked.
    let Some(cells) = field.reconstruct(goal) else {
        return Err(PathBlocked);
    };
    let total = field.cost_of(&goal).unwrap_or(PathCost::ZERO).to_tu();
    Ok(Path::new(cells, total))
}

/// Every `(cell, level)` reachable from `start` within `budget` TU, each paired with
/// the cheapest accumulated cost to reach it — the bounded Dijkstra distance-field
/// FLOOD (C2).
///
/// The SAME relaxation core ([`relax`]) as [`find_path`], with NO goal and NO
/// heuristic (`h ≡ 0` — Dijkstra's native flood shape, ADR-0005 OQ-2): it seeds
/// `start` at cost `0` and expands outward, but PRUNES ([`StopRule::Prune`]) any
/// settled node whose accumulated cost EXCEEDS `budget` — that node is kept in the
/// reachable set (it is itself within budget, being settled at its own cost) but its
/// neighbours are not paid for beyond the budget. The result is every cell whose
/// cheapest route cost is `≤ budget`.
///
/// The returned collection is SORTED by the `(z, y, x)` cell key (C4) — the
/// underlying `HashMap` order never leaks, so two replays over the same snapshot +
/// budget yield a byte-identical reachable set. This is what the GTW-357
/// reachable-range / move-preview overlay consumes.
///
/// PURE (`bevy-traps.md` #7): a free function over the borrowed snapshot — no
/// `&mut World`, no system, no RNG, no per-query rebuild (C5).
#[must_use]
pub fn reachable_within(
    start: CellLevel,
    budget: Tu,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
) -> Vec<(CellLevel, Tu)> {
    let grids = SearchGrids {
        grid,
        links,
        tuning,
    };
    let budget_cost = PathCost::new(u32::from(*budget));
    let field = relax(
        start,
        grids,
        // No goal → no heuristic (the flood is goal-free Dijkstra).
        |_cell| PathCost::ZERO,
        // A node strictly OVER budget is settled (in-set) but not expanded; a node
        // within budget expands normally.
        move |_cell, cost| {
            if cost > budget_cost {
                StopRule::Prune
            } else {
                StopRule::Expand
            }
        },
    );

    // Yield every settled cell within budget, sorted by the deterministic cell key.
    // A pruned boundary node can sit just over budget (it was settled before its
    // over-budget cost was checked), so filter the final set to `≤ budget` here.
    field
        .settled_sorted()
        .into_iter()
        .filter(|(_, cost)| *cost <= budget_cost)
        .map(|(cell, cost)| (cell, cost.to_tu()))
        .collect()
}
