//! Terrain authoring mode — draft and save.
mod draft;
mod error;
mod inputs;
mod load;
mod picks;
mod save;

#[cfg(test)]
mod tests;

pub use draft::TerrainDraft;
pub use error::SaveTerrainError;
pub use inputs::{ArmorInput, HpInput};
pub use picks::{FootfallChoice, TerrainKindChoice, offered_graphic_roles};
pub use save::{draft_to_terrain_def, serialize_terrain_def};
#[cfg(debug_assertions)]
pub use save::{write_terrain, write_terrain_in};
