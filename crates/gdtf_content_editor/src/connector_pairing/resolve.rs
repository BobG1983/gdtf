//! Recognise a staircase by the tag its own def carries.

use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainTag, TerrainUuid};

/// Whether `tile` is a staircase, read off the def's own stair tag.
#[must_use]
pub fn is_stair(registry: &TerrainDefRegistry, tile: &TerrainUuid) -> bool {
    registry
        .def(tile)
        .is_some_and(|def| def.tags.contains(&TerrainTag::Stair))
}
