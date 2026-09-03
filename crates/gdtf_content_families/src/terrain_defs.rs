//! Terrain definition content family.

use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainDefRegistry};

/// Loads `*.terrain_def.ron` files into [`TerrainDefRegistry`].
pub struct TerrainDefsFamily;

impl ContentFamily for TerrainDefsFamily {
    type Spec = TerrainDef;
    type Registry = TerrainDefRegistry;

    const EXTENSION: &'static str = "terrain_def.ron";
    const FOLDER: &'static str = "content/terrain";

    fn insert_member(
        registry: &mut TerrainDefRegistry,
        _stem: Option<ContentFileStem>,
        def: &TerrainDef,
    ) -> Option<ContentMemberKey> {
        registry.insert(def.key, def.clone());
        Some(ContentMemberKey::new((*def.key).to_string()))
    }
}
