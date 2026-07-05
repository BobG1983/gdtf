//! The coarse 3D occupancy grid [`OccupancyGrid`] — the model's `(cell, level)`
//! collision/query surface plus its append-only [`DestroyedCover`] exclusion set
//! and the stair-cell eye-offset map ([`StairEyeOffset`] / [`OccupancyGrid::is_stair_cell`]).

mod blocking;
mod extent;
mod stairs;
mod storage;
mod types;
mod vision;

pub use stairs::StairEyeOffset;
pub use storage::OccupancyGrid;
pub use types::{DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancySlot};
