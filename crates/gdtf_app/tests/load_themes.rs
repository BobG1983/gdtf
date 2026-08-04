//! only binds it to [`ThemeDefsFamily`] with the KNOWN authored [`ThemeUuid`].
mod load_suite;

use bevy::asset::uuid::Uuid;
use gdtf_app::test_support::AppState;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::TerrainUuid,
};
use gdtf_content_families::ThemeDefsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

const LOAD_SAFETY_NET: u32 = 10_000;

const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0184_0a90_0001))
}

const fn industrial_hive_default_floor() -> TerrainUuid {
    TerrainUuid::new(Uuid::from_u128(0x0184_0a91_0004))
}

impl FamilyLoadContract for ThemeDefsFamily {
    const EXPECTED_MEMBERS: &'static [&'static str] = &["industrial_hive"];

    fn is_empty(registry: &UuidThemeRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &UuidThemeRegistry, label: &str) -> bool {
        match label {
            "industrial_hive" => registry.def(&industrial_hive_theme()).is_some(),
            _ => false,
        }
    }
}

#[test]
fn theme_defs_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<ThemeDefsFamily>();
}

#[test]
fn load_does_not_leave_without_a_uuid_theme_registry() {
    suite::load_gates_on_registry::<ThemeDefsFamily>();
}

#[test]
fn real_asset_resolves_uuid_theme_registry() {
    suite::real_asset_resolves_registry::<ThemeDefsFamily>();
}

#[test]
fn real_asset_theme_default_floor_cross_reference_resolves() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<UuidThemeRegistry>();
    assert!(
        registry.is_some(),
        "the real per-theme folder load must insert a UuidThemeRegistry within the safety-net \
         budget",
    );
    if let Some(registry) = registry {
        assert_eq!(
            registry.default_floor(&industrial_hive_theme()),
            Some(industrial_hive_default_floor()),
            "the resolved theme's default_floor must reference the authored terrain UUID",
        );
    }
}
