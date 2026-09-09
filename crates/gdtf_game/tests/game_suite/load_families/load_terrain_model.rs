//! Terrain model load: real assets resolve `TerrainDefRegistry` and `UuidThemeRegistry` by UUID.
use std::path::PathBuf;

use cobalt_test_utils::{LoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_game::test_support::{AppState, load_released};

const fn known_terrain_uuid() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0801))
}

const fn known_theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_08a1))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("terrain_model_root")
}

#[test]
fn real_asset_resolves_new_terrain_and_theme_registries_by_uuid() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        fixture_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<TerrainDefRegistry>(&mut app);
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app);

    if let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved TerrainDefRegistry must carry the authored fixture def(s)",
        );
        assert!(
            registry.def(&known_terrain_uuid()).is_some(),
            "the registry must resolve the known authored TerrainUuid (keyed by the def's own \
             UUID, not the filename)",
        );
    }

    if let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved UuidThemeRegistry must carry the authored fixture theme def(s)",
        );
        assert!(
            registry.def(&known_theme_uuid()).is_some(),
            "the registry must resolve the known authored ThemeUuid (keyed by the def's own \
             UUID)",
        );
        assert_eq!(
            registry.default_floor(&known_theme_uuid()),
            Some(known_terrain_uuid()),
            "the resolved theme's default_floor must reference the authored terrain UUID",
        );
    }

    advance_until(&mut app, load_released);
    assert!(
        app.world().get_resource::<TerrainDefRegistry>().is_some()
            && app.world().get_resource::<UuidThemeRegistry>().is_some(),
        "both new registries must be present after Load releases (the gate waited for them — not hand-seeded defaults)",
    );
}
