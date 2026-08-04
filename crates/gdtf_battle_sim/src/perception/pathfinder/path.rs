//! Path result types.

use crate::{ganger::Tu, metric::CellLevel};

/// Total path cost in TU units (u32 accumulator).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PathCost(u32);

impl PathCost {
    /// Zero cost.
    pub const ZERO: Self = Self(0);

    /// Wrap a raw cost value.
    #[must_use]
    pub const fn new(cost: u32) -> Self {
        Self(cost)
    }

    /// Add one step's TU cost.
    #[must_use]
    pub fn add_step(self, step: Tu) -> Self {
        Self(self.0 + u32::from(*step))
    }

    /// Narrow to a `Tu` value, saturating at `u8::MAX`.
    #[must_use]
    pub const fn to_tu(self) -> Tu {
        let narrowed = if self.0 > u8::MAX as u32 {
            u8::MAX
        } else {
            self.0 as u8
        };
        Tu::new(narrowed)
    }
}

/// A found path: cells, per-step costs, and total cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    cells: Vec<CellLevel>,
    steps: Vec<Tu>,
    total: Tu,
}

impl Path {
    /// Build a path from cells, step costs, and total.
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>, steps: Vec<Tu>, total: Tu) -> Self {
        Self {
            cells,
            steps,
            total,
        }
    }

    /// Cells along the path including start and goal.
    #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

    /// Per-step TU costs.
    #[must_use]
    pub fn steps(&self) -> &[Tu] {
        &self.steps
    }

    /// Total TU cost.
    #[must_use]
    pub const fn total(&self) -> Tu {
        self.total
    }

    /// First cell, if any.
    #[must_use]
    pub fn start(&self) -> Option<CellLevel> {
        self.cells.first().copied()
    }

    /// Last cell, if any.
    #[must_use]
    pub fn goal(&self) -> Option<CellLevel> {
        self.cells.last().copied()
    }

    /// Number of cells.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the path is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// Sentinel returned when no path exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathBlocked;
