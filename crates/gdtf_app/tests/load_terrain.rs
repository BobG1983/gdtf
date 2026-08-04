//! Load terrain into [`TerrainDefsFamily`] by known authored UUIDs.
//! Value-agnostic: presence and UUID resolution only.
mod load_suite;

use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use gdtf_content_families::TerrainDefsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for TerrainDefsFamily {
    const EXPECTED_MEMBERS: &'static [&'static str] = &["deck_floor", "bulkhead_wall"];

    fn is_empty(registry: &TerrainDefRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &TerrainDefRegistry, label: &str) -> bool {
        let uuid = match label {
            "deck_floor" => Uuid::from_u128(0x0184_0a91_0004),
            "bulkhead_wall" => Uuid::from_u128(0x0184_0a91_0002),
            _ => return false,
        };
        registry.def(&TerrainUuid::new(uuid)).is_some()
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
