//! Load terrain defs: non-empty registry from the real folder; no pinned stems.
use gdtf_app::test_support::AppState;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainSimKind};
use gdtf_content_families::TerrainDefsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

use super::load_suite;

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

#[test]
fn a_shipped_emplacement_names_at_least_one_entry_side() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app);

    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        unreachable!("the wait above returns once the registry is in the world");
    };
    let emplacements: Vec<&Vec<_>> = registry
        .defs()
        .filter_map(|(_, def)| match &def.sim_kind {
            TerrainSimKind::Emplacement { entry_sides, .. } => Some(entry_sides),
            TerrainSimKind::Wall { .. }
            | TerrainSimKind::Cover { .. }
            | TerrainSimKind::Slab { .. } => None,
        })
        .collect();
    assert!(
        !emplacements.is_empty(),
        "the real terrain folder (assets/content/terrain) must load at least one Emplacement \
         def — none in the registry means the folder load or its salvage dropped one",
    );
    assert!(
        emplacements.iter().any(|sides| !sides.is_empty()),
        "at least one of the {} shipped emplacement def(s) must name an entry side, or the game \
         ships no mount a ganger can enter",
        emplacements.len(),
    );
}
