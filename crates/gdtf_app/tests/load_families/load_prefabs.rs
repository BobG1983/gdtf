//! Prefab registry load from real assets; gate waits for it before leaving Load.
use gdtf_app::test_support::{AppState, load_released};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

#[test]
fn real_asset_resolves_prefab_registry() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<PrefabRegistry>(&mut app);

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
