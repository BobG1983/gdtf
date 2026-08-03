//! Assemble player and enemy spawn placements.

mod pick;
mod place;

pub use place::{PlacedPrefab, Placement, assemble_placement, assemble_placement_with};
pub(in crate::lifecycle::procgen) use place::{place_enemy, place_player};
