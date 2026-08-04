//! Pick nearest target and advance cell toward a goal.

use bevy::prelude::Entity;

use crate::{
    ganger::Tu,
    metric::{Cell, CellDistance, CellLevel, Level},
};

/// Candidate enemy for AI targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiTarget {
    /// Enemy entity.
    pub entity: Entity,
    /// Enemy cell.
    pub cell:   Cell,
    /// Enemy level.
    pub level:  Level,
}

impl AiTarget {
    /// Build a target.
    #[must_use]
    pub const fn new(entity: Entity, cell: Cell, level: Level) -> Self {
        Self {
            entity,
            cell,
            level,
        }
    }
}

#[must_use]
fn chebyshev_xy(a: Cell, b: Cell) -> CellDistance {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    CellDistance::new(dx.max(dy))
}

#[must_use]
fn order_key(from_cell: Cell, from_level: Level, target: AiTarget) -> (u32, u32, u8, i32, i32) {
    let distance = chebyshev_xy(from_cell, target.cell);
    let level_gap = u32::from((*from_level).abs_diff(*target.level));
    (
        *distance,
        level_gap,
        *target.level,
        target.cell.y,
        target.cell.x,
    )
}

/// Nearest candidate by Chebyshev distance, then level gap, then cell order.
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

/// Best reachable cell that gets closer to `goal` than `start`.
#[must_use]
pub fn plan_advance(
    start: CellLevel,
    goal: Cell,
    reachable: &[(CellLevel, Tu)],
) -> Option<CellLevel> {
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
