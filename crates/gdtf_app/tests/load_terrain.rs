//! Load terrain defs: non-empty registry from the real folder; no pinned stems.
mod load_suite;

use gdtf_battle_sim::terrain::def::TerrainDefRegistry;
use gdtf_content_families::TerrainDefsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for TerrainDefsFamily {
    fn is_empty(registry: &TerrainDefRegistry) -> bool {
        registry.is_empty()
    }
}

#[test]
fn terrain_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<TerrainDefsFamily>();
}

#[test]
fn load_does_not_leave_without_a_terrain_def_registry() {
    suite::load_gates_on_registry::<TerrainDefsFamily>();
}

#[test]
fn real_asset_resolves_terrain_def_registry_by_uuid() {
    suite::real_asset_resolves_registry::<TerrainDefsFamily>();
}
