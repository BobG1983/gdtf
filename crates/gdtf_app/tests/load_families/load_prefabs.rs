//! Prefab registry load from real assets; gate waits for it before leaving Load.
use gdtf_app::test_support::{AppState, load_released};
use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabName, PrefabRegistry,
        SpawnRole, ThemeUuid,
    },
    terrain::def::TerrainDefRegistry,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

const PREFAB_FAMILY: &str = "PrefabRegistry";

/// The three prefabs the game ships, by the file stem the loader keys them under.
const SHIPPED_PREFABS: [&str; 3] = ["enemy_deployment", "player_deployment", "entry_room"];

const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

#[test]
fn real_asset_resolves_prefab_registry() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<PrefabRegistry>(&mut app);

    advance_until_resource_exists::<ContentValidationDone>(&mut app);
    let malformed: Vec<String> = app
        .world()
        .get_resource::<ContentIntegrityReport>()
        .map(|report| {
            report
                .findings()
                .iter()
                .filter_map(|finding| match finding {
                    ContentFinding::MalformedFile { path, family, .. }
                        if **family == PREFAB_FAMILY =>
                    {
                        Some((**path).clone())
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        malformed.is_empty(),
        "no shipped prefab may be reported malformed — every authored field the loader \
         does not find must serde-default. Malformed: {malformed:?}",
    );

    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the migrated (non-empty) prefabs — an \
             empty registry means resolve_prefabs is broken (fell back to the empty default)",
        );

        let size = GridSize::new(GridWidth::new(12), GridHeight::new(12), GridLevels::new(1));
        assert!(
            size.is_ok(),
            "the 12x12x1 deployment footprint must be a valid GridSize"
        );
        let Some(size) = size.ok() else { return };
        let key = PrefabKey::new(industrial_hive_theme(), size, SpawnRole::Player);
        let bucket = registry.prefabs_for(&key);
        assert!(
            !bucket.is_empty(),
            "the (IndustrialHive, 12x12, Player) bucket must hold the migrated player-deployment \
             prefab — enumeration by (theme, size, role) is the C2 contract",
        );
        for prefab in bucket {
            assert!(
                !prefab.spec().placements.is_empty(),
                "the migrated player-deployment prefab must carry >= 1 placement",
            );
        }
    }

    advance_until(&mut app, load_released);
    assert!(
        app.world().get_resource::<PrefabRegistry>().is_some(),
        "a PrefabRegistry must be present when Load reaches Intro (the gate clause waited \
         for it — not a hand-seeded default)",
    );
}

#[test]
fn every_shipped_placement_names_a_terrain_def_the_registry_holds() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<PrefabRegistry>(&mut app);
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app);

    let Some(prefabs) = app.world().get_resource::<PrefabRegistry>() else {
        unreachable!("the wait above returns once the prefab registry is in the world");
    };
    let Some(terrain) = app.world().get_resource::<TerrainDefRegistry>() else {
        unreachable!("the wait above returns once the terrain registry is in the world");
    };
    for wanted in SHIPPED_PREFABS {
        assert!(
            prefabs
                .iter()
                .any(|prefab| prefab.name() == &PrefabName::new(wanted.to_owned())),
            "the loaded registry must hold the shipped prefab `{wanted}`, or the walk below \
             passes on zero iterations",
        );
    }

    let mut visited: usize = 0;
    for prefab in prefabs.iter() {
        for placement in &prefab.spec().placements {
            visited += 1;
            assert!(
                terrain.def(&placement.piece).is_some(),
                "prefab `{}` places `{}`, and the terrain registry holds no def for it, so the \
                 assembled level would carry an empty cell where a piece was authored",
                **prefab.name(),
                *placement.piece,
            );
        }
    }
    assert!(
        visited > 0,
        "the shipped prefabs must carry at least one terrain placement between them, or this \
         walk asserts nothing",
    );
}
