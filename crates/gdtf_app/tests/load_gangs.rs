//! only binds it to [`GangsFamily`] with the authored gang stems. The
mod load_suite;

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangerName};
use gdtf_content_families::GangsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for GangsFamily {
        const EXPECTED_MEMBERS: &'static [&'static str] = &["gang_0", "gang_1"];

    fn is_empty(registry: &GangRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &GangRegistry, label: &str) -> bool {
        registry.roster(&GangName::new(label.to_owned())).is_some()
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
fn real_asset_gang_rosters_hold_their_members() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<GangRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<GangRegistry>();
    assert!(
        registry.is_some(),
        "the real gangs folder load must insert a GangRegistry within the safety-net budget",
    );
    let Some(registry) = registry else {
        return;
    };

    let gang_0 = registry.roster(&GangName::new("gang_0".to_owned()));
    assert!(
        gang_0
            .and_then(|roster| roster.member(&GangerName::new("Alex Mercer".to_owned())))
            .is_some(),
        "the registry must hold gang_0 (keyed by gang_0.gang.ron's stem) with member \"Alex Mercer\"",
    );

    let gang_1 = registry.roster(&GangName::new("gang_1".to_owned()));
    assert!(
        gang_1
            .and_then(|roster| roster.member(&GangerName::new("Vex 1".to_owned())))
            .is_some(),
        "the registry must hold gang_1 (keyed by gang_1.gang.ron's stem) with member \"Vex 1\"",
    );
    assert!(
        gang_1
            .and_then(|roster| roster.member(&GangerName::new("Vex 2".to_owned())))
            .is_some(),
        "the registry's gang_1 must also hold member \"Vex 2\"",
    );
}
