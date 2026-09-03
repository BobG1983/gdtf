//! Coarse occupancy grid: who and what occupies each cell, plus path and vision blocking.

mod grid;
mod input;
mod kind;
mod neighbours;
mod path_blocking;
mod vision_blocking;

#[cfg(test)]
mod test;

#[cfg(test)]
mod path_blocking_test;

#[cfg(test)]
mod vision_blocking_test;

pub use grid::{
    Blocked, BodyOcclusion, GRID_HEIGHT, GRID_WIDTH, OccludesVision, OccupancyGrid, OccupancySlot,
    PathBlocked, StairCell, StairEyeOffset,
};
pub use input::{OccupancyInput, OccupantPlacement, TerrainPlacement};
pub use kind::TerrainKind;
pub use neighbours::pathable_neighbors;
pub use path_blocking::{PathBlocking, project_path_blocking};
pub use vision_blocking::{VisionBlocking, VisionOccluderChanged, project_vision_blocking};
