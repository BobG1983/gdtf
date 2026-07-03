//! The **pure** AI decision functions — ECS-free, deterministic-without-RNG, and
//! unit-testable in isolation (GTW-70 §C / §D.2).
//!
//! These carry the AI's two ordering decisions, separated from the ECS brain
//! ([`enemy_ai_turn`](crate::ai::enemy_ai_turn)) so they can be tested as plain functions
//! over plain data: [`pick_nearest`] (the target / advance-goal pick, §C) and
//! [`plan_advance`] (the reposition step, §D.2). Both are total functions of their inputs
//! with a `(level, y, x)` total-order tie-break at every choice point — NO `HashMap` /
//! [`Entity`](bevy::prelude::Entity)-id iteration order, NO RNG (`docs/combat/resolution.md`
//! — the minimal AI is deterministic-without-RNG, GTW-70 §E).

use bevy::prelude::Entity;

use crate::{
    ganger::Tu,
    metric::{Cell, CellLevel, Level},
};

/// A candidate the AI reasons about — an opposing ganger (or its cell as an advance goal),
/// keyed by its [`Entity`] handle and its `(cell, level)` (GTW-70).
///
/// A transparent grouping of an opposing ganger's identity + position the pure decision
/// functions order over. The [`Entity`] is framework plumbing (the no-bare-types carve-out
/// for a Bevy handle); [`cell`](AiTarget::cell) / [`level`](AiTarget::level) are the named
/// metric newtypes. `Copy` so the pick functions can return one by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiTarget {
    /// The opposing ganger's entity — carried so the FIRE emission can name its shooter's
    /// target and the advance can pick its cell as the goal.
    pub entity: Entity,
    /// The opposing ganger's ground cell `(x, y)`.
    pub cell:   Cell,
    /// The opposing ganger's storey.
    pub level:  Level,
}

impl AiTarget {
    /// Build a candidate from an opposing ganger's entity + `(cell, level)`.
    #[must_use]
    pub const fn new(entity: Entity, cell: Cell, level: Level) -> Self {
        Self {
            entity,
            cell,
            level,
        }
    }
}

/// The 2D **Chebyshev** distance between two ground cells — `max(|dx|, |dy|)`, the z axis
/// ignored (GTW-70 §C; mirrors the `can_see` range metric in
/// [`crate::los`](crate::los::can_see)).
///
/// The range metric the AI orders targets by — the same `max(|dx|, |dy|)` the engagement
/// gate's range disc uses, so "nearest" agrees with "in view-range". Saturating into the
/// `u32` magnitude on the pathological out-of-grid delta (defined, never a panic).
#[must_use]
fn chebyshev_xy(a: Cell, b: Cell) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    dx.max(dy)
}

/// The total-order key for ranking [`candidates`](AiTarget) relative to a shooter at
/// `(from_cell, from_level)` — `(Chebyshev, |Δlevel|, level, y, x)` (GTW-70 §C).
///
/// Lexicographic: nearest by 2D Chebyshev first, then the smallest storey gap, then the
/// `(level, y, x)` of the candidate cell as the final deterministic tie-break. Because no
/// two gangers share a `(cell, level)`, the final triple is unique per candidate, so the
/// whole key is a TOTAL order — the pick is a pure function of game state, never dependent
/// on `Entity`-id or `HashMap` iteration order.
#[must_use]
fn order_key(from_cell: Cell, from_level: Level, target: AiTarget) -> (u32, u32, u8, i32, i32) {
    let distance = chebyshev_xy(from_cell, target.cell);
    let level_gap = u32::from((*from_level).abs_diff(*target.level));
    (
        distance,
        level_gap,
        *target.level,
        target.cell.y,
        target.cell.x,
    )
}

/// Pick the **nearest** candidate to a shooter at `(from_cell, from_level)` — minimum 2D
/// Chebyshev, tie-broken by `(|Δlevel|, level, y, x)` (GTW-70 §C).
///
/// The §C target / advance-goal selector: among `candidates` it returns the one minimising
/// `order_key` — a TOTAL order, so the result is a deterministic pure function of game
/// state regardless of the slice's order (no `HashMap` / `Entity`-id dependence). `None` on
/// an empty list (no opposing ganger to engage or advance toward). Lowest-HP /
/// best-hit-chance scoring is **GTW-71**; minimal uses nearest.
#[must_use]
pub fn pick_nearest(
    from_cell: Cell,
    from_level: Level,
    candidates: &[AiTarget],
) -> Option<AiTarget> {
    candidates
        .iter()
        .copied()
        .min_by_key(|target| order_key(from_cell, from_level, *target))
}

/// Plan a one-commit reposition toward `goal` over the AI's affordable reachable set —
/// the [`reachable_within`](crate::pathfinder::reachable_within) cell that strictly
/// reduces the 2D Chebyshev distance to `goal` the most (GTW-70 §D.2).
///
/// `reachable` is the `(cell, cost)` set [`reachable_within`](crate::pathfinder::reachable_within)
/// produced from `start` over the AI's omniscient move fog (so a yielded cell is, by
/// construction, one [`find_path`](crate::pathfinder::find_path) accepts and affords — the
/// emit-⇒-accept guarantee that keeps the enemy turn terminating, GTW-70 §D). Among them
/// this picks the cell minimising `(Chebyshev-to-goal, cost, level, y, x)` — closest to the
/// goal, cheapest, then the `(level, y, x)` total-order tie-break.
///
/// Returns the chosen [`CellLevel`] ONLY when it **strictly** reduces the distance from
/// `start` (the §B clause-2 "strictly reduces distance" rule); otherwise [`None`] (HOLD) —
/// including the degenerate cases where `start` is already the closest reachable cell, or
/// the shooter is co-located with the goal. So an emitted move always makes progress, which
/// (with the fire path's `can_engage` pre-check) is what bounds the turn's termination.
#[must_use]
pub fn plan_advance(
    start: CellLevel,
    goal: Cell,
    reachable: &[(CellLevel, Tu)],
) -> Option<CellLevel> {
    // The canonical CellLevel::cell accessor (GTW-565) for every ground-plane read.
    let start_distance = chebyshev_xy(start.cell(), goal);
    let best = reachable.iter().copied().min_by_key(|(cell, cost)| {
        (
            chebyshev_xy(cell.cell(), goal),
            u32::from(**cost),
            cell.z,
            cell.y,
            cell.x,
        )
    })?;
    let (best_cell, _) = best;
    let best_distance = chebyshev_xy(best_cell.cell(), goal);
    if best_distance < start_distance {
        Some(best_cell)
    } else {
        None
    }
}
