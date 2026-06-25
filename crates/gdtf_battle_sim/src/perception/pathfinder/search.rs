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

use bevy::prelude::Entity;

use super::{
    core::{SearchGrids, StopRule, relax},
    path::{Path, PathBlocked, PathCost},
    planning::PlanningView,
};
use crate::{
    ganger::Tu,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    terrain::floor::FloorCostGrid,
    tuning::{CombatTuning, MoveCost},
    vertical::VerticalLinkGraph,
    visibility::FactionRelation,
};

/// The minimum positive per-step move cost on the grid — `4`, the A\* admissibility
/// floor (GTW-396: also the minimum [`MoveCost`] an authored floor piece is allowed to
/// carry; `setup_battle` rejects any floor piece below this).
///
/// A typed [`MoveCost`] constant — never a bare integer — so the heuristic site and the
/// setup-validation site read the SAME single source of truth without an `as` cast at
/// either end (GTW-396 structure pass).
///
/// Load-bearing for the A\* heuristic's admissibility (ADR-0005 OQ-2 / §"Context":
/// "The minimum positive move cost on the grid is 4"): scaling the planar
/// step-distance by the MINIMUM per-step cost guarantees the heuristic never
/// overestimates the true cheapest route, which is what keeps A\* admissible (and so
/// optimal). Documented as a constant rather than read from tuning because it is the
/// FLOOR of the cost scale — the heuristic must bound from below by the cheapest
/// possible step, never the actual (possibly dearer) authored floor cost; using the
/// hard floor stays admissible even if the `default_floor` cost is tuned upward.
///
/// Exported `pub` so [`setup_battle`](crate::situation::setup_battle) can validate
/// floor piece costs against this same floor — ensuring the authored data and the
/// heuristic never drift apart (GTW-396 Decision B, `FloorCostBelowMinimum` error).
pub const MIN_MOVE_COST: MoveCost = MoveCost::new(4);

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
    // `MIN_MOVE_COST` is a typed `MoveCost` (wrapping `u8`); deref + widen to `u32` for
    // the planar-distance product — the single source of truth, no `as` cast.
    PathCost::new(steps * u32::from(*MIN_MOVE_COST))
}

/// Find the cheapest legal route from `start` to `goal` over the occupancy grid +
/// vertical-link graph, **routing only through visibility-routable cells** (GTW-353),
/// or [`PathBlocked`] if no such route exists (C1).
///
/// **Visibility gating (GTW-353, C1 / C2).** `planning` ([`PlanningView`]) gates which
/// candidate cells the search may route INTO: an UNSEEN (never-seen) cell is
/// non-routable, so the search routes AROUND it and a goal reachable ONLY by crossing
/// UNSEEN is [`PathBlocked`]; EXPLORED (remembered) cells remain routable (the
/// user-ratified OQ-5 interpretation). Within routable cells, walls / floor / standing
/// cover always block, an own-squad ganger always blocks, and an enemy blocks iff its
/// cell is squad-VISIBLE (see [`PlanningView::is_routable`]). The cost model + the
/// deterministic tie-break are unchanged — the gate only removes edges.
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
/// stitching"). The cost model is read per planar step from the
/// [`FloorCostGrid`] (the per-cell floor move cost, GTW-396 Decision C1 — the sole
/// step-cost source, replacing the coarse `tuning.move_costs` table) plus the flat
/// [`LinkTu`](crate::tuning::LinkTu) from `tuning` at a link hop; the search NEVER
/// charges TU — it plans and totals, and the per-step writers charge at commit (§48).
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
/// walkable, visibility-routable grid + links (a typed no-route result — never a
/// panic, never an empty [`Path`]) — including when every route to `goal` must cross
/// an UNSEEN cell (C1).
pub fn find_path<R>(
    start: CellLevel,
    goal: CellLevel,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
    floor_costs: &FloorCostGrid,
    planning: &PlanningView<'_, R>,
) -> Result<Path, PathBlocked>
where
    R: Fn(Entity) -> FactionRelation,
{
    let grids = SearchGrids {
        grid,
        links,
        tuning,
        floor_costs,
        planning,
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
    // The per-step entry costs ALIGNED to `cells[1..]`, derived from the SAME settled
    // accumulated-cost deltas the relaxation built (NOT re-run cost math): the cost to
    // enter `cells[i + 1]` is `cost(cells[i + 1]) − cost(cells[i])`, the exact
    // `PathCost::add_step` increment along the route. Their sum is the goal's settled
    // cost (the route total), bit-for-bit — the §48 identity GTW-355's walk charges.
    let steps = field.step_costs(&cells);
    let total = field.cost_of(&goal).unwrap_or(PathCost::ZERO).to_tu();
    Ok(Path::new(cells, steps, total))
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
/// **Visibility gating (GTW-353, C1 / C2).** `planning` ([`PlanningView`]) gates the
/// flood: an UNSEEN (never-seen) cell is non-routable, so it is **never yielded** and
/// the flood does not pay to expand through it; EXPLORED (remembered) cells remain
/// reachable (the user-ratified OQ-5 interpretation). Within routable cells the same
/// within-routable blocking applies as [`find_path`] (walls / floor / cover always
/// block, an own-squad ganger always blocks, an enemy blocks iff squad-VISIBLE — see
/// [`PlanningView::is_routable`]).
///
/// The returned collection is SORTED by the `(z, y, x)` cell key (C4) — the
/// underlying `HashMap` order never leaks, so two replays over the same snapshot +
/// budget yield a byte-identical reachable set. This is what the GTW-357
/// reachable-range / move-preview overlay consumes.
///
/// PURE (`bevy-traps.md` #7): a free function over the borrowed snapshot — no
/// `&mut World`, no system, no RNG, no per-query rebuild (C5).
#[must_use]
pub fn reachable_within<R>(
    start: CellLevel,
    budget: Tu,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
    floor_costs: &FloorCostGrid,
    planning: &PlanningView<'_, R>,
) -> Vec<(CellLevel, Tu)>
where
    R: Fn(Entity) -> FactionRelation,
{
    let grids = SearchGrids {
        grid,
        links,
        tuning,
        floor_costs,
        planning,
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
