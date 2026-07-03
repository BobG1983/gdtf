//! The pure positional corner-detection geometry — [`corner_lean`] + the
//! [`PEEK_LEAN`] sub-cell magnitude (GTW-406 §A / §B).
//!
//! A ganger "hugs a corner" when a cardinal neighbour is a blocker (a wall or
//! undestroyed cover — anything [`OccupancyGrid::is_blocked`] reports) AND that wall
//! **ends** beside the ganger (exactly one cell past the wall, along the wall's own
//! axis, is open). The lean then points *along* the wall toward that open end — never
//! toward the blocker, so a peeked eye can never be nudged THROUGH the wall.
//!
//! Purely positional and **target-agnostic** (the 2026-06-24 "automatic + positional"
//! ruling): no [`Target`](crate::los::Target) is consulted, so for a target on the
//! *opposite* side of the corner the lean is "wrong" — a known limitation of a
//! positional peek, deferred to a future target-aware consumer. The lone-pillar /
//! flat-wall non-peek cases (below) fall out of the same target-agnostic rule.
//!
//! Pure over an [`OccupancyGrid`] + a [`CellLevel`]: render-free, RNG-free,
//! deterministic, zero ECS — unit-testable with hand-built grids.

use bevy::math::Vec2;

use crate::{
    los::PeekOffset,
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
};

/// The four cardinal neighbour offsets, in a **fixed priority order**
/// (`N, E, S, W`) so the lean a multi-blocker cell resolves to is deterministic and
/// replay-stable — never iteration-order dependent (GTW-406 §A).
const CARDINALS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

/// The sub-cell lean magnitude (sim units) — how far toward the open cell edge the
/// eye is nudged (GTW-406 §B).
///
/// `0.4` from cell-centre `0.5` puts the eye at `0.9` / `0.1` on the leaned axis,
/// strictly inside `[corner, corner+1)` — GTW-393-test-validated.
///
/// **No-bare-types carve-out (geometry constant, not a domain value).** This is a
/// fixed sub-cell *geometry* constant — it is defined BY the `0.5` half-cell metric
/// (the eye must land strictly inside its own cell, so the lean is bounded by the
/// half-cell `0.5`), not a balance knob a designer would retune. Per the GTW-406 gate's
/// fidelity + structure lenses it is therefore NOT a domain newtype candidate and NOT a
/// `tuning.ron` tunable: promoting it to a `CombatTuning` field would wrongly expand the
/// tuning surface with a value that is geometrically fixed. It lives as a private
/// `const` here, the single place the half-cell lean is expressed.
pub(super) const PEEK_LEAN: f32 = 0.4;

/// The single-axis wall-peek lean for `cell` as a [`PeekOffset`] —
/// `(±`[`PEEK_LEAN`]`, 0)` / `(0, ±`[`PEEK_LEAN`]`)` toward a corner's open edge, or
/// [`PeekOffset::default`] when `cell` hugs no corner (GTW-406 §A).
///
/// For each cardinal `d` in the fixed [`CARDINALS`] priority order:
///
/// 1. `wall = cell + d`; skip unless [`OccupancyGrid::is_blocked`]`(&wall)`.
/// 2. The wall's own axis is the perpendicular `p = (-d.y, d.x)` (a 90° rotation of
///    `d`, so `p` is never `d` — the lean is always *along* the wall, never into it).
/// 3. `plus_open = !is_blocked(wall + p)`, `minus_open = !is_blocked(wall - p)`.
/// 4. **Exactly one end open** (`plus_open != minus_open`) → a true corner: lean
///    toward the open end (`p` if `plus_open`, else `-p`), scaled by [`PEEK_LEAN`].
/// 5. Otherwise (both ends open = a lone pillar; both blocked = a mid-flat-wall) this
///    wall is not a corner — continue to the next cardinal.
///
/// No cardinal qualifies → [`PeekOffset::default`] (no peek). An out-of-range neighbour
/// reads as not-blocked (open) via [`OccupancyGrid::is_blocked`]'s graceful out-of-range
/// path — no panic.
pub(super) fn corner_lean(cell: CellLevel, grid: &OccupancyGrid) -> PeekOffset {
    for (dx, dy) in CARDINALS {
        let wall = offset_in_plane(cell, dx, dy);
        if !grid.is_blocked(&wall) {
            continue;
        }
        // The wall's own axis: a 90° rotation of the cardinal `d` (never `d` itself),
        // so the lean runs ALONG the wall toward an open end — structurally impossible
        // to nudge the eye THROUGH the blocker.
        let (px, py) = (-dy, dx);
        let plus_open = !grid.is_blocked(&offset_in_plane(wall, px, py));
        let minus_open = !grid.is_blocked(&offset_in_plane(wall, -px, -py));
        // A true corner has EXACTLY ONE end of this wall open (XOR). Both open = a lone
        // pillar (target-dependent, deferred); both blocked = a mid-flat-wall (no edge).
        if plus_open != minus_open {
            let (lx, ly) = if plus_open { (px, py) } else { (-px, -py) };
            #[expect(
                clippy::cast_precision_loss,
                reason = "lx/ly are a unit step in {-1, 0, 1}; the f32 conversion is exact"
            )]
            return PeekOffset::new(Vec2::new(lx as f32, ly as f32) * PEEK_LEAN);
        }
    }
    PeekOffset::default()
}

/// `at` offset by `(dx, dy)` cells on its OWN storey — the same-level planar neighbour
/// the corner scan probes. The storey index is carried verbatim (a peek corner is a
/// planar wall relationship; slabs live in the `SurfaceGrid` and never affect the
/// planar [`OccupancyGrid::is_blocked`]).
fn offset_in_plane(at: CellLevel, dx: i32, dy: i32) -> CellLevel {
    // The storey via the canonical CellLevel::level accessor (GTW-565).
    CellLevel::new(Cell::new(at.x + dx, at.y + dy), at.level())
}
