//! Staged procgen cursor and footprint records.

mod cursor;
mod footprint;

pub(in crate::lifecycle::procgen) use cursor::ProcgenCursor;
pub use cursor::{ProcgenStage, StagedProcgenRegistries};
pub use footprint::{PlacedFootprint, PlacementRole};
