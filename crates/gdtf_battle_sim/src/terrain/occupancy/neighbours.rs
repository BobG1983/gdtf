//! Pathable-neighbour **enumeration** (E7 · GTW-12b): the pure
//! [`pathable_neighbors`] read over the [`OccupancyGrid`] — the same-storey
//! 8-connected planar neighbours a ganger can step to from a `(cell, level)`,
//! each priced at its step cost.
//!
//! This is a PURE function over the existing grid + the move-cost table — no new
//! traversal state, no new resource, no `&mut World`. It is one half of the
//! neighbour model ADR-0005 (Accepted) ratifies; the cross-storey other half is
//! GTW-351's [`traversable_links`](crate::vertical::traversable_links), and the
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

use std::f32::consts::SQRT_2;

use crate::{
    ganger::Tu,
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, TerrainKind},
    tuning::MoveCosts,
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
/// A PURE read over the [`OccupancyGrid`] (and the [`MoveCosts`] table for the
/// step price) — no `&mut World`, no system, no RNG. It enumerates the eight
/// orthogonal + diagonal planar offsets of `origin` (ADR-0005's ratified
/// 8-connected adjacency), keeps the walkable ones, prices each, and yields them in
/// the canonical `(z, y, x)` cell-key order.
///
/// **Signature — AC1 vs AC5 reconciled to `(CellLevel, `[`Tu`]`)`.** The ticket's
/// AC1 sketches `-> impl Iterator<Item = CellLevel>` while AC5 adds the octile step
/// COST. Rather than split the cost into a sibling function, the cost is BUNDLED
/// into each yielded edge, mirroring GTW-351's
/// [`traversable_links`](crate::vertical::traversable_links) (which yields
/// `(CellLevel, `[`Tu`]`)`). This gives the route core (GTW-352) ONE uniform
/// `(neighbour, cost)` edge interface across planar steps and vertical hops, so the
/// search relaxes both edge kinds identically — the reason the ticket prefers
/// bundling.
///
/// **Step cost (C2).** The price of a step is a function of the ENTERED cell's
/// terrain move cost (looked up via [`MoveCosts::cost`] on the destination's
/// [`TerrainKind`]):
///
/// - **Orthogonal** (N/E/S/W) — the entered cell's `move_cost` unchanged.
/// - **Diagonal** (the four corners) — the OCTILE approximation
///   `round(move_cost × √2)` as an integer [`Tu`] (e.g. open `4 → round(5.66) = 6`,
///   cover `6 → round(8.49) = 8`, matching the ADR-0005 example). Derived from the
///   existing `move_cost` and the ratified `√2` factor — NOT a new tunable. The
///   rounding is `f32::round` (round-half-away-from-zero) on the `move_cost × √2`
///   product, then narrowed to the `u8` [`Tu`] inner.
///
/// **No corner-cutting (C3, `AT_LEAST_ONE_WALKABLE`).** A diagonal step is
/// EXCLUDED if BOTH of its two shared-edge orthogonal neighbours are blocked — a
/// unit cannot "phase" diagonally through the corner where two blocked cells meet.
/// For a `(dx, dy)` diagonal the two shared-edge orthogonals are the `(dx, 0)` and
/// `(0, dy)` cells; if both are [`is_blocked`](OccupancyGrid::is_blocked) the
/// diagonal is dropped (an orthogonal step is never corner-cut, so this gate
/// applies to diagonals only).
///
/// **Walkability (C4).** A neighbour is included iff it is IN-BOUNDS and
/// [`is_blocked`](OccupancyGrid::is_blocked) returns `false` — which already treats
/// a DESTROYED-cover cell as walkable (the grid's destroyed-cover exclusion set) and
/// excludes standing walls / cover. Out-of-bounds offsets are dropped (the grid's
/// graceful bounds check: an out-of-range cell reads as un-blocked but is never
/// in-bounds, so it is filtered out explicitly here).
///
/// **Determinism (C5).** Neighbours are yielded in the canonical `(z, y, x)`
/// cell-key order — the same total order [`auto_select`] uses — by iterating
/// [`PLANAR_OFFSETS`] (which is pre-sorted in that order for a same-storey
/// neighbourhood), so two replays over the same grid emit byte-identical edge
/// sequences.
///
/// An `origin` whose neighbours are all blocked / out-of-bounds yields an empty
/// iterator (never a panic). This is the planar GATE + cost only — it does not
/// assemble a route (GTW-352).
///
/// [`auto_select`]: https://linear.app/robert-gardner/issue/GTW-255
pub fn pathable_neighbors<'a>(
    origin: CellLevel,
    grid: &'a OccupancyGrid,
    move_costs: &'a MoveCosts,
) -> impl Iterator<Item = (CellLevel, Tu)> + 'a {
    // The origin's storey — every planar neighbour shares it (same-storey, dz = 0).
    let level = origin.z;
    PLANAR_OFFSETS.into_iter().filter_map(move |(dx, dy)| {
        let neighbour = planar_neighbour(origin, level, dx, dy);
        // C4: in-bounds AND not blocked. `slot` is `None` for an out-of-bounds
        // cell (so the cell is dropped); `is_blocked` excludes standing walls /
        // cover while INCLUDING destroyed-cover cells.
        if grid.slot(&neighbour).is_none() || grid.is_blocked(&neighbour) {
            return None;
        }
        let diagonal = dx != 0 && dy != 0;
        // C3 (no corner-cutting): a diagonal is illegal when BOTH its shared-edge
        // orthogonal neighbours are blocked. Orthogonal steps are never corner-cut.
        if diagonal && corner_is_cut(origin, level, dx, dy, grid) {
            return None;
        }
        // C2: the entered cell's terrain move cost; diagonals pay the octile
        // `round(move_cost × √2)`, orthogonals pay it unchanged.
        let cost = step_cost(grid.terrain(&neighbour), *move_costs, diagonal);
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
/// BOTH of those are [`is_blocked`](OccupancyGrid::is_blocked) — the unit would
/// have to slip through the gap where two blocked cells meet at a corner. If at
/// least one is walkable the diagonal is fine. Only called for true diagonals
/// (`dx != 0 && dy != 0`).
fn corner_is_cut(origin: CellLevel, level: i32, dx: i32, dy: i32, grid: &OccupancyGrid) -> bool {
    let side_a = planar_neighbour(origin, level, dx, 0);
    let side_b = planar_neighbour(origin, level, 0, dy);
    grid.is_blocked(&side_a) && grid.is_blocked(&side_b)
}

/// The [`Tu`] cost of stepping onto a cell of terrain `terrain` — orthogonal pays
/// the entered cell's [`MoveCost`](crate::tuning::MoveCost) unchanged, a `diagonal`
/// pays the OCTILE `round(move_cost × √2)` (C2).
///
/// The single cost seam: it looks the entered cell's per-terrain cost up via
/// [`MoveCosts::cost`], then — for a diagonal — multiplies by [`SQRT_2`] and
/// rounds. The rounding is `f32::round` (round-half-away-from-zero) on the
/// `move_cost × √2` product, narrowed back to the `u8` the [`Tu`] inner carries
/// (`open 4 → 6`, `cover 6 → 8`, per ADR-0005). Derived from the existing
/// `move_cost` and the ratified `√2` factor — NOT a new tunable.
fn step_cost(terrain: TerrainKind, move_costs: MoveCosts, diagonal: bool) -> Tu {
    let orthogonal = *move_costs.cost(terrain);
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
