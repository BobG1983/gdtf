//! The floor move-cost surface (GTW-396) — the per-cell [`FloorCostGrid`] resource
//! the pathfinder reads to price terrain steps, replacing the coarse per-kind
//! [`MoveCosts`](crate::tuning::MoveCosts) table.

mod cost_grid;

pub use cost_grid::FloorCostGrid;
