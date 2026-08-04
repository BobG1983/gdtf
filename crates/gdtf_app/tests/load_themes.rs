//! Load themes into [`ThemeDefsFamily`].
//! Value-agnostic: registry presence and structural properties only.
mod load_suite;

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::level::UuidThemeRegistry;
use gdtf_content_families::ThemeDefsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for ThemeDefsFamily {
    fn is_empty(registry: &UuidThemeRegistry) -> bool {
        registry.is_empty()
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
fn real_asset_themes_declare_terrain_and_default_floor() {
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
    let Some(registry) = registry else {
        return;
    };

    assert!(
        !registry.is_empty(),
        "shipped themes folder must yield at least one theme",
    );
    assert!(
        registry.defs().all(|(_uuid, def)| !def.terrain.is_empty()),
        "every shipped theme must list at least one terrain uuid (structural property)",
    );
    assert!(
        registry
            .defs()
            .all(|(uuid, def)| { registry.default_floor(uuid) == Some(def.default_floor) }),
        "every shipped theme must expose its default_floor via lookup",
    );
}
