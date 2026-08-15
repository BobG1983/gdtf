//! Migrated prefabs: shipped content resolves with placements and role defaults.
use gdtf_app::test_support::{AppState, load_released};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

fn footprint(width: u8, height: u8) -> Option<GridSize> {
    GridSize::new(
        GridWidth::new(width),
        GridHeight::new(height),
        GridLevels::new(1),
    )
    .ok()
}

#[test]
fn shipped_migrated_prefabs_resolve_with_placements_and_role_default() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<PrefabRegistry>(&mut app);

    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the migrated shipped prefabs — an empty \
             registry means resolve_prefabs fell back to the empty default",
        );

        let Some(size_3) = footprint(3, 3) else {
            return;
        };
        let Some(size_12) = footprint(12, 12) else {
            return;
        };
        let theme = industrial_hive_theme();

        let buckets = [
            (size_3, SpawnRole::Fill, "3x3 entry_room"),
            (size_12, SpawnRole::Player, "12x12 player_deployment"),
            (size_12, SpawnRole::Enemy, "12x12 enemy_deployment"),
        ];
        for (size, role, label) in buckets {
            let key = PrefabKey::new(theme, size, role);
            let bucket = registry.prefabs_for(&key);
            assert!(
                !bucket.is_empty(),
                "the migrated {label} bucket (IndustrialHive ThemeUuid, role {role:?}) must hold its prefab(s)",
            );
            for prefab in bucket {
                assert!(
                    !prefab.spec().placements.is_empty(),
                    "the migrated {label} prefab must carry >= 1 placement (the old walls/scatter \
                     refs mapped to migrated TerrainUuids)",
                );
                assert_eq!(
                    prefab.spec().role,
                    role,
                    "the migrated {label} prefab must carry its authored role {role:?}",
                );
            }
        }
    }

    advance_until(&mut app, load_released);
}
