//! Occupancy grid storage, blocking, stairs, and vision helpers.

mod blocking;
mod bodies;
mod extent;
mod stairs;
mod storage;
mod types;
mod vision;

pub use bodies::BodyOcclusion;
pub use stairs::{StairCell, StairEyeOffset};
pub use storage::OccupancyGrid;
pub use types::{Blocked, GRID_HEIGHT, GRID_WIDTH, OccludesVision, OccupancySlot, PathBlocked};
