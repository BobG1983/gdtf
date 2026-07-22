//! The procgen **engine** — the unified step primitive both drivers loop (GTW-732).
//!
//! [`cursor`] holds the [`ProcgenCursor`] whose one `step` advances procgen by exactly one
//! unit of work (place a prefab / finalize the fill / emit), the [`ProcgenStage`] identity it
//! returns, and the [`StagedProcgenRegistries`] borrow-bundle it takes;
//! [`footprint`] holds the render-free [`PlacedFootprint`] / [`PlacementRole`] the schematic
//! overview draws. `mod.rs` is wiring-only.

mod cursor;
mod footprint;

pub(in crate::lifecycle::procgen) use cursor::ProcgenCursor;
pub use cursor::{ProcgenStage, StagedProcgenRegistries};
pub use footprint::{PlacedFootprint, PlacementRole};
