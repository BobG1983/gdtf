//! Occupancy grid storage, blocking, stairs, and vision helpers.

mod blocking;
mod extent;
mod stairs;
mod storage;
mod types;
mod vision;

pub use stairs::{StairCell, StairEyeOffset};
pub use storage::OccupancyGrid;
pub use types::{
    Blocked, CoverDestroyed, DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccludesVision,
    OccupancySlot, PathBlocked,
};
