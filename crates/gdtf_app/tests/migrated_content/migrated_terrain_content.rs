//! Terrain load: shipped migrated terrain and theme content resolve by UUID.
use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainTag, TerrainUuid},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

const LOAD_SAFETY_NET: u32 = 10_000;

const MIGRATED_TERRAIN_DEF_COUNT: usize = 27;

const MIGRATED_THEME_COUNT: usize = 3;

const fn terrain_uuid(low: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(low))
}

const fn theme_uuid(low: u128) -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(low))
}

const fn industrial_hive_flat_eight() -> [TerrainUuid; 8] {
    [
        terrain_uuid(0x0184_0a91_0001),
        terrain_uuid(0x0184_0a91_0002),
        terrain_uuid(0x0184_0a91_0003),
        terrain_uuid(0x0184_0a91_0004),
        terrain_uuid(0x0184_0a91_0005),
        terrain_uuid(0x0184_0a91_0006),
        terrain_uuid(0x0184_0a91_0007),
        terrain_uuid(0x0184_0a91_0008),
    ]
}

const fn migrated_themes() -> [(ThemeUuid, TerrainUuid); 3] {
    [
        (theme_uuid(0x0184_0a90_0001), terrain_uuid(0x0184_0a91_0004)),
        (theme_uuid(0x0184_0a90_0002), terrain_uuid(0x0184_0a92_0001)),
        (theme_uuid(0x0184_0a90_0003), terrain_uuid(0x0184_0a93_0001)),
    ]
}

const fn bulkhead_wall_uuid() -> TerrainUuid {
    terrain_uuid(0x0184_0a91_0002)
}

#[test]
fn shipped_migrated_terrain_and_theme_content_resolves_by_uuid() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<TerrainDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved TerrainDefRegistry must carry the migrated shipped terrain defs",
        );
        assert_eq!(
            registry.len(),
            MIGRATED_TERRAIN_DEF_COUNT,
            "the registry must hold every migrated terrain def (reconciled flat 8 + 4 + 4, plus \
             the EW-wall companions 2 + 1 + 1) — a count mismatch means a piece was \
             silently dropped (C2)",
        );
        for uuid in industrial_hive_flat_eight() {
            assert!(
                registry.def(&uuid).is_some(),
                "the reconciled industrial_hive flat-8 piece {uuid:?} must resolve — none of the \
                 source-of-truth flat 8 may be dropped (C2)",
            );
        }

        if let Some(wall) = registry.def(&bulkhead_wall_uuid()) {
            assert!(
                matches!(wall.sim_kind, TerrainSimKind::Wall { .. }),
                "the migrated bulkhead_wall must be a Wall sim_kind",
            );
            assert!(
                wall.tags.contains(&TerrainTag::BlocksVision)
                    && wall.tags.contains(&TerrainTag::BlocksPathfinding),
                "the authored sim-side wall tags must round-trip through the loader (C4)",
            );
        }
    }

    if let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved UuidThemeRegistry must carry the migrated shipped theme defs",
        );
        assert_eq!(
            registry.len(),
            MIGRATED_THEME_COUNT,
            "the registry must hold all 3 migrated themes (industrial_hive, underhive, \
             sump_waste)",
        );
        for (theme, expected_floor) in migrated_themes() {
            assert!(
                registry.def(&theme).is_some(),
                "the migrated theme {theme:?} must resolve by its ThemeUuid (C2)",
            );
            assert_eq!(
                registry.default_floor(&theme),
                Some(expected_floor),
                "the migrated theme {theme:?} must resolve its authored default_floor TerrainUuid \
                 (C2 cross-reference consistency)",
            );
            if let Some(terrain) = app.world().get_resource::<TerrainDefRegistry>() {
                assert!(
                    terrain.def(&expected_floor).is_some(),
                    "the theme {theme:?} default_floor TerrainUuid must resolve to a migrated \
                     terrain def (no dangling reference)",
                );
            }
        }
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once every folder \
         (incl. the migrated per-theme terrain/) resolves; last AppState was {:?}",
        app_state(&app),
    );
}
