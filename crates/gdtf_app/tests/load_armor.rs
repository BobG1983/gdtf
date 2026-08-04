//! Load armor into [`ArmorFamily`] by authored member keys.
//! Value-agnostic: registry presence only; magnitudes are tuning data.
mod load_suite;

use bevy::app::Startup;
use gdtf_app::test_support::{AppState, app_state, load_released, seed_load_fallbacks};
use gdtf_battle_sim::armor::ArmorRegistry;
use gdtf_content_families::ArmorFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for ArmorFamily {
    fn is_empty(registry: &ArmorRegistry) -> bool {
        registry.is_empty()
    }
}

#[test]
fn armor_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<ArmorFamily>();
}

#[test]
fn load_does_not_leave_without_an_armor_registry() {
    suite::load_gates_on_registry::<ArmorFamily>();
}

#[test]
fn real_asset_resolves_armor_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<ArmorFamily>();
}

#[test]
fn seeded_startup_does_not_shadow_real_armor_resolution() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_systems(Startup, seed_load_fallbacks);

    advance_until_resource_exists::<ArmorRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<ArmorRegistry>() {
        assert!(
            !registry.is_empty(),
            "the real folder resolve must populate the registry, not leave the empty seed",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with the Startup seed present, Load must still release to Intro (or beyond) once the \
         real armor resolves; last AppState was {:?}",
        app_state(&app),
    );
}
