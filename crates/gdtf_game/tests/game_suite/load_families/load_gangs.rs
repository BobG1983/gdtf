//! Load gangs into [`GangsFamily`] by authored gang stems.
//! Value-agnostic: presence and roster shape only; spawn cells come from deploy.
use cobalt_test_utils::{LoadTestAppBuilder, advance_until_resource_exists};
use gdtf_battle_sim::ganger::GangRegistry;
use gdtf_content_families::GangsFamily;
use gdtf_game::test_support::AppState;
use load_suite::suite::{self, FamilyLoadContract};

use super::load_suite;

impl FamilyLoadContract for GangsFamily {
    fn is_empty(registry: &GangRegistry) -> bool {
        registry.is_empty()
    }
}

#[test]
fn gangs_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<GangsFamily>();
}

#[test]
fn load_does_not_leave_without_a_gang_registry() {
    suite::load_gates_on_registry::<GangsFamily>();
}

#[test]
fn real_asset_resolves_gang_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<GangsFamily>();
}

#[test]
fn real_asset_gang_rosters_hold_members() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    advance_until_resource_exists::<GangRegistry>(&mut app);

    let registry = app.world().get_resource::<GangRegistry>();
    assert!(
        registry.is_some(),
        "the real gangs folder load must insert a GangRegistry",
    );
    let Some(registry) = registry else {
        return;
    };

    assert!(
        !registry.is_empty(),
        "shipped gangs folder must yield at least one gang",
    );
    assert!(
        registry
            .iter()
            .any(|(_name, roster)| !roster.members.is_empty()),
        "at least one shipped gang must hold a non-empty roster (property, not named members)",
    );
}
