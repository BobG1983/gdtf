//! Load terrain defs: non-empty registry from the real folder; no pinned stems.
use cobalt_test_utils::{LoadTestAppBuilder, advance_until_resource_exists};
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainSimKind};
use gdtf_content_families::TerrainDefsFamily;
use gdtf_game::test_support::AppState;
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
fn some_shipped_terrain_def_authors_an_on_death_list() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app);

    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        unreachable!("the wait above returns once the registry is in the world");
    };
    assert!(
        registry.defs().any(|(_, def)| !def.on_death.is_empty()),
        "at least one shipped terrain def must author a non-empty on_death list, or the \
         migration to a list emptied them",
    );
}

// The art the eight deleted orientation twins carried, now on the survivors' view rows.
const MIGRATED_SPRITES: [&str; 5] = [
    "wall_ew",
    "door_ew",
    "stair_ns_down",
    "stair_ew_up",
    "stair_ew_down",
];

// The survivors that absorbed a twin's art, so each owes more than one distinct key.
const ABSORBED_A_TWIN: [&str; 6] = [
    "Bulkhead Wall",
    "Heavy Bulkhead",
    "Corroded Bulkhead",
    "Tunnel Wall",
    "Bulkhead Door",
    "Deck Stair",
];

#[test]
fn the_orientation_twins_art_lives_on_the_survivors_view_rows() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app);

    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        unreachable!("the wait above returns once the registry is in the world");
    };
    for wanted in MIGRATED_SPRITES {
        assert!(
            registry
                .defs()
                .any(|(_, def)| def.views.iter().any(|row| &**row.sprite == wanted)),
            "no shipped def names `{wanted}` on any view row, so deleting the twin that used to \
             carry it dropped that art out of the shipped content",
        );
    }

    for name in ABSORBED_A_TWIN {
        let Some((_, def)) = registry.defs().find(|(_, def)| &**def.display_name == name) else {
            unreachable!("the shipped registry must hold `{name}`");
        };
        let mut keys: Vec<&str> = def.views.iter().map(|row| &**row.sprite).collect();
        keys.sort_unstable();
        keys.dedup();
        assert!(
            keys.len() > 1,
            "`{name}` absorbed an orientation twin's art, so its view rows must carry more than \
             one distinct sprite key; they carry {keys:?}",
        );
    }
}

#[test]
fn a_shipped_emplacement_names_at_least_one_entry_side() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
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
