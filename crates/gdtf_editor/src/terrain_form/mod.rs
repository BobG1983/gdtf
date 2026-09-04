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
pub use load::{load_candidates, terrain_source};
pub use picks::{FootfallChoice, TerrainKindChoice, offers_view_expander, view_rows};
pub use save::{draft_to_terrain_def, serialize_terrain_def};
#[cfg(feature = "mcp")]
pub use save::{write_terrain, write_terrain_in};
