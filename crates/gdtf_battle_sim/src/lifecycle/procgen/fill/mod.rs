//! Fill free space after player/enemy placement.

mod cursor;
mod outcome;
mod passes;
mod pipeline;

pub(in crate::lifecycle::procgen) use cursor::{FillCursor, FillStep};
pub use outcome::FilledPlacement;
pub use pipeline::{fill_placement, fill_placement_with};
