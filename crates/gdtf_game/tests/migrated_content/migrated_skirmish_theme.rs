//! Skirmish load: shipped situation names a migrated theme UUID that resolves.
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{AppState, load_released};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

#[test]
fn shipped_skirmish_names_a_migrated_theme_uuid_that_resolves() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<LoadedSituation>(&mut app);
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app);

    let authored_theme_uuid = app
        .world()
        .get_resource::<LoadedSituation>()
        .map(|loaded| loaded.theme);

    assert_eq!(
        authored_theme_uuid,
        Some(industrial_hive_theme()),
        "skirmish.ron must author the migrated IndustrialHive theme (a ThemeUuid), resolved \
         through the situation loader path (C5)",
    );

    if let (Some(theme_uuid), Some(registry)) = (
        authored_theme_uuid,
        app.world().get_resource::<UuidThemeRegistry>(),
    ) {
        assert!(
            registry.def(&theme_uuid).is_some(),
            "the migrated theme skirmish.ron names must resolve in the UuidThemeRegistry (C5)",
        );
    }

    advance_until(&mut app, load_released);
}
