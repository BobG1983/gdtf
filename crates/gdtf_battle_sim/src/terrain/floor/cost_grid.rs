//! The [`FloorCostGrid`] resource — the per-cell move-cost surface the pathfinder
//! reads for terrain step cost (GTW-396).
//!
//! Replaces the coarse [`MoveCosts`](crate::tuning::MoveCosts) open/cover/wall table
//! in the pathfinder hot path. The grid holds a **default** `(cell, level)` cost
//! (from the situation's `default_floor` terrain piece) plus a sparse **override**
//! map for cells that author a different floor piece (the `floors` list). The
//! pathfinder reads [`FloorCostGrid::cost`] for every planar step INSTEAD of
//! `move_costs.cost(grid.terrain(&neighbour))`.

use bevy::{platform::collections::HashMap, prelude::Resource};

use crate::{metric::CellLevel, tuning::MoveCost};

/// The **per-cell floor move-cost surface** — the single cost source the pathfinder
/// reads for terrain step cost (GTW-396, Decision B / C1).
///
/// Holds a default [`MoveCost`] (the situation's `default_floor` piece's cost,
/// applied to any cell NOT in the override map) and a sparse `HashMap` of
/// `(cell, level)` → [`MoveCost`] overrides for cells that author a different floor
/// piece (the situation's `floors` list). The pathfinder calls
/// [`FloorCostGrid::cost`] for every planar DESTINATION cell — this replaces
/// `move_costs.cost(grid.terrain(&neighbour))` as the sole floor cost source.
///
/// A Bevy [`Resource`] — inserted at `setup_battle` and removed at teardown, sharing
/// the [`BattleInProgress`](crate::battle::BattleInProgress) lifetime.
///
/// Every authored floor cost is pinned ≥ [`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST)
/// by the `setup_battle` validation (the `FloorCostBelowMinimum` abort), which rejects
/// any floor piece below the A\* admissibility floor — so the heuristic stays admissible
/// without this grid needing to re-derive that minimum.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct FloorCostGrid {
    /// The default move cost — applies to any `(cell, level)` not in `overrides`.
    default:   MoveCost,
    /// Sparse per-cell move-cost overrides for cells with a non-default floor piece.
    overrides: HashMap<CellLevel, MoveCost>,
}

impl FloorCostGrid {
    /// Build a floor-cost grid from its default cost and an iterator of per-cell
    /// overrides — the constructor `setup_battle` calls after resolving the situation's
    /// `default_floor` and `floors` lists against the terrain-definition registry.
    #[must_use]
    pub fn new(
        default: MoveCost,
        overrides: impl IntoIterator<Item = (CellLevel, MoveCost)>,
    ) -> Self {
        Self {
            default,
            overrides: overrides.into_iter().collect(),
        }
    }

    /// The move cost for the given `(cell, level)` — the override if one is authored,
    /// otherwise the default.
    ///
    /// This is the pathfinder's SOLE cost source: `pathable_neighbors` calls `cost(&neighbour)`
    /// for every planar destination cell instead of `move_costs.cost(grid.terrain(&neighbour))`.
    #[must_use]
    pub fn cost(&self, at: &CellLevel) -> MoveCost {
        self.overrides.get(at).copied().unwrap_or(self.default)
    }
}
