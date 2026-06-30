//! Pathable-neighbour **enumeration** (E7 · GTW-12b): the pure
//! [`pathable_neighbors`] read over the [`OccupancyGrid`] — the same-storey
//! 8-connected planar neighbours a ganger can step to from a `(cell, level)`,
//! each priced at its step cost.
//!
//! This is a PURE function over the existing grid + the [`FloorCostGrid`] —
//! no new traversal state, no new resource, no `&mut World`. It is one half of
//! the neighbour model ADR-0005 (Accepted) ratifies; the cross-storey other half
//! is GTW-351's [`traversable_links`](crate::vertical::traversable_links), and the
//! two are deliberately shaped the same way (each yields `(CellLevel, `[`Tu`]`)`
//! edges) so the route core (GTW-352) can consume a single edge interface.
//!
//! What ADR-0005 ratifies and this enforces:
//!
//! - **8-connected** — orthogonal + diagonal (the user-ratified adjacency model).
//! - **Octile diagonal cost** — `round(orthogonal_move_cost × √2)` (C2), so a
//!   diagonal step costs ≈ √2 of the entered cell's orthogonal cost. Flat
//!   same-cost diagonals are rejected (they make diagonal dashes strictly best).
//! - **No corner-cutting** — the `AT_LEAST_ONE_WALKABLE` rule: a diagonal step is
//!   illegal if BOTH its shared-edge orthogonal neighbours are blocked (C3).
//! - **Deterministic order** — neighbours are emitted in the canonical `(z, y, x)`
//!   cell-key order (the `auto_select` `cell_order_key` precedent), so a replay is
//!   byte-equal (C5).
//!
//! Scope: same-storey planar adjacency + step cost ONLY. Cross-storey hops are
//! GTW-351; route assembly (search over these edges) is GTW-352.
//!
//! ## GTW-396 cost-seam change
//!
//! The step cost is now read from [`FloorCostGrid::cost`] for the DESTINATION cell
//! instead of `move_costs.cost(grid.terrain(&neighbour))`. This replaces the coarse
//! per-[`TerrainKind`] table (open=4, cover=6, wall=8) with the per-cell authored
//! floor cost the situation's `default_floor` / `floors` list supplies.
//! Walls and standing cover are still blocked by [`OccupancyGrid::is_blocked`] —
//! their `FloorCostGrid` entries (if any) are never reached as walkable destinations.

use std::f32::consts::SQRT_2;

use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    terrain::floor::FloorCostGrid,
    tuning::MoveCost,
};

/// The eight planar `(dx, dy)` step offsets of an 8-connected neighbourhood, in
/// the canonical `(z, y, x)` cell-key order relative to the origin.
///
/// All eight share the origin's storey (`dz = 0`), so ordering by the destination
/// cell key reduces to ordering by `(dy, dx)` — lower row first, then lower column.
/// Emitting offsets in this fixed order makes the neighbour iteration deterministic
/// (C5) WITHOUT a post-hoc sort: the four `dy = -1` rows precede the two `dy = 0`
/// rows precede the three `dy = +1` rows, and within each row `dx` ascends. The
/// `dx == 0 && dy == 0` origin itself is deliberately absent (a cell is not its own
/// neighbour).
const PLANAR_OFFSETS: [(i32, i32); 8] = [
    (-1, -1), // SW row (dy = -1), dx ascending
    (0, -1),
    (1, -1),
    (-1, 0), // same row (dy = 0)
    (1, 0),
    (-1, 1), // NW row (dy = +1)
    (0, 1),
    (1, 1),
];

/// The same-storey 8-connected planar neighbours of `origin` over `grid` that are
/// IN-BOUNDS and NOT blocked, each paired with the [`Tu`] cost of stepping ONTO it
/// (C1).
///
/// A PURE read over the [`OccupancyGrid`] (and the [`FloorCostGrid`] for the step
/// price) — no `&mut World`, no system, no RNG. It enumerates the eight orthogonal +
/// diagonal planar offsets of `origin` (ADR-0005's ratified 8-connected adjacency),
/// keeps the walkable ones, prices each from the [`FloorCostGrid`], and yields them
/// in the canonical `(z, y, x)` cell-key order.
///
/// **Step cost (C2, GTW-396).** The price of a step is a function of the ENTERED
/// cell's floor cost from [`FloorCostGrid::cost`] — NOT the coarse per-kind
/// `MoveCosts` table. This is the GTW-396 Decision B / C1 rewire:
///
/// - **Orthogonal** (N/E/S/W) — the entered cell's `floor_costs.cost(&neighbour)` unchanged.
/// - **Diagonal** (the four corners) — the OCTILE approximation
///   `round(move_cost × √2)` as an integer [`Tu`] (e.g. default floor `4 → round(5.66) = 6`,
///   matching the ADR-0005 example). The rounding is `f32::round`
///   (round-half-away-from-zero), then narrowed to the `u8` [`Tu`] inner.
///
/// **Movement-cost factor (GTW-444 C3).** The base step cost (orthogonal or octile) is
/// then scaled by the mover's `factor` ([`MovementCostFactor`] — the "Hampered"
/// slowdown) and rounded UP (`ceil`): a factor `>= 1.0` NEVER reduces a step below its
/// base terrain cost. The factor is PER-GANGER (passed in by the caller, read from the
/// mover's [`InflictedInjuries`](crate::injuries::InflictedInjuries)), NOT per-tile — the
/// SAME scaling the committed walk's per-step charge applies, so a previewed path cost
/// equals the TU actually charged (preview==charge). An uninjured mover passes
/// [`MovementCostFactor::IDENTITY`] (`1.0`), leaving the cost at the base terrain cost.
///
/// **No corner-cutting (C3 GTW-396, `AT_LEAST_ONE_WALKABLE`).** A diagonal step is
/// EXCLUDED if BOTH of its two shared-edge orthogonal neighbours are blocked — a
/// unit cannot "phase" diagonally through the corner where two blocked cells meet.
///
/// **Walkability (C4).** A neighbour is included iff it is IN-BOUNDS and
/// [`is_blocked`](OccupancyGrid::is_blocked) returns `false` — which already treats
/// a DESTROYED-cover cell as walkable and excludes standing walls / cover.
///
/// **Determinism (C5).** Neighbours are yielded in the canonical `(z, y, x)`
/// cell-key order by iterating [`PLANAR_OFFSETS`] (pre-sorted); the factor scaling is a
/// pure `ceil(cost × factor)`, deterministic for a fixed `(cost, factor)`.
///
/// An `origin` whose neighbours are all blocked / out-of-bounds yields an empty
/// iterator (never a panic). This is the planar GATE + cost only — it does not
/// assemble a route (GTW-352).
///
/// [`auto_select`]: https://linear.app/robert-gardner/issue/GTW-255
pub fn pathable_neighbors<'a>(
    origin: CellLevel,
    grid: &'a OccupancyGrid,
    floor_costs: &'a FloorCostGrid,
    factor: MovementCostFactor,
) -> impl Iterator<Item = (CellLevel, Tu)> + 'a {
    // The origin's storey — every planar neighbour shares it (same-storey, dz = 0).
    let level = origin.z;
    PLANAR_OFFSETS.into_iter().filter_map(move |(dx, dy)| {
        let neighbour = planar_neighbour(origin, level, dx, dy);
        // C4: in-bounds AND not PATH-blocked (GTW-501 D1/C2). `slot` is `None` for an
        // out-of-bounds cell (so the cell is dropped); `is_path_blocked` reads the
        // TAG-DERIVED path-blocking surface — NOT the kind-based `is_blocked` vision reads
        // — excluding cells with a `BlocksPathfinding` marker while still INCLUDING
        // destroyed-cover cells (the surface mirrors the destroyed-cover exclusion, C5).
        if grid.slot(&neighbour).is_none() || grid.is_path_blocked(&neighbour) {
            return None;
        }
        let diagonal = dx != 0 && dy != 0;
        // C3 (no corner-cutting): a diagonal is illegal when BOTH its shared-edge
        // orthogonal neighbours are blocked. Orthogonal steps are never corner-cut.
        if diagonal && corner_is_cut(origin, level, dx, dy, grid) {
            return None;
        }
        // C2 (GTW-396): the entered cell's floor cost from FloorCostGrid;
        // diagonals pay the octile `round(move_cost × √2)`, orthogonals unchanged.
        // GTW-444: then scale by the mover's MovementCostFactor (ceil, never below base).
        let base = step_cost(floor_costs.cost(&neighbour), diagonal);
        let cost = scale_by_factor(base, factor);
        Some((neighbour, cost))
    })
}

/// The `(cell, level)` of the planar neighbour at offset `(dx, dy)` on the origin's
/// storey `level`.
///
/// A small helper so the offset → `CellLevel` construction reads once. Same-storey
/// (`dz = 0`), so the storey is `level` unchanged; the cell is `origin`'s cell
/// shifted by `(dx, dy)`. Reads `origin.x`/`origin.y` through [`CellLevel`]'s
/// [`Deref`](std::ops::Deref) to its inner `IVec3`.
fn planar_neighbour(origin: CellLevel, level: i32, dx: i32, dy: i32) -> CellLevel {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "level is origin.z, a storey index in 0..MAX_LEVELS (8) by the grid's own \
                  construction (an out-of-range origin is filtered by the bounds check at the \
                  call site), so the i32 -> u8 narrowing cannot truncate or sign-flip"
    )]
    let level = Level::new(level as u8);
    CellLevel::new(Cell::new(origin.x + dx, origin.y + dy), level)
}

/// Whether the diagonal step `(dx, dy)` from `origin` would CUT a corner — the
/// `AT_LEAST_ONE_WALKABLE` test of C3.
///
/// The two cells a `(dx, dy)` diagonal shares an edge with are the orthogonal
/// `(dx, 0)` and `(0, dy)` neighbours. The diagonal is a corner-cut (illegal) iff
/// BOTH of those are [`is_path_blocked`](OccupancyGrid::is_path_blocked) — the unit would
/// have to slip through the gap where two PATH-blocking cells meet at a corner. The
/// corner test reads the SAME tag-derived path-blocking surface as the walkability gate
/// above (GTW-501 D1/C2), not the kind-based `is_blocked` vision reads. If at least one is
/// path-walkable the diagonal is fine. Only called for true diagonals
/// (`dx != 0 && dy != 0`).
fn corner_is_cut(origin: CellLevel, level: i32, dx: i32, dy: i32, grid: &OccupancyGrid) -> bool {
    let side_a = planar_neighbour(origin, level, dx, 0);
    let side_b = planar_neighbour(origin, level, 0, dy);
    grid.is_path_blocked(&side_a) && grid.is_path_blocked(&side_b)
}

/// The [`Tu`] cost of stepping onto a cell with floor cost `floor_cost` — orthogonal
/// pays `floor_cost` unchanged; a `diagonal` pays the OCTILE `round(floor_cost × √2)`.
///
/// GTW-396 Decision B / C1: the cost is now fed directly as a [`MoveCost`] from
/// [`FloorCostGrid::cost`] at the call site, rather than looked up via
/// `move_costs.cost(terrain)` — the caller passes the resolved per-cell cost. The
/// octile math is unchanged: `f32::round(move_cost × √2)`, narrowed to `u8`.
fn step_cost(floor_cost: MoveCost, diagonal: bool) -> Tu {
    let orthogonal = *floor_cost;
    if !diagonal {
        return Tu::new(orthogonal);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "orthogonal is a u8 move cost (<= 255), so move_cost * √2 < 361 and its rounded \
                  value fits a u8; the product is non-negative so the u8 cast cannot sign-flip \
                  (and `.round()` has already discarded the fractional part)"
    )]
    let octile = (f32::from(orthogonal) * SQRT_2).round() as u8;
    Tu::new(octile)
}

/// Scale a base per-step [`Tu`] cost by the mover's [`MovementCostFactor`] — the GTW-444
/// "Hampered" slowdown — rounding UP (`ceil`).
///
/// `out = ceil(base × factor)`. The rounding is DETERMINISTIC `ceil` (the documented C3
/// choice — picked over round-half-up because it guarantees a factor `>= 1.0` can NEVER
/// reduce a step below its base terrain cost: `ceil(base × 1.0) == base`, and any factor
/// `> 1.0` rounds up, so the floor of the scaled cost is always `>= base`). The IDENTITY
/// factor (`1.0`) returns the base unchanged. The SAME helper is the single scaling rule
/// the committed walk reuses, so preview == charge (C3). Saturates at `u8::MAX` rather
/// than wrapping for an absurd factor (such a route is unaffordable and never committed).
fn scale_by_factor(base: Tu, factor: MovementCostFactor) -> Tu {
    // The IDENTITY factor (1.0) is a no-op fast path — and keeps an uninjured mover's cost
    // bit-identical to the pre-GTW-444 base, with no float round-trip (C6 identity).
    if factor == MovementCostFactor::IDENTITY {
        return base;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "base is a u8 Tu (<= 255) and an authored MovementCostFactor is >= 1.0; the \
                  product is clamped to u8::MAX before the cast (so it cannot truncate or \
                  wrap) and is non-negative (so the u8 cast cannot sign-flip); `.ceil()` has \
                  already discarded the fractional part"
    )]
    let scaled = {
        let raw = (f32::from(*base) * factor.raw()).ceil();
        if raw > f32::from(u8::MAX) {
            u8::MAX
        } else {
            raw as u8
        }
    };
    Tu::new(scaled)
}
