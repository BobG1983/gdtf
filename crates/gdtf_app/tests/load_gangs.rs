//! GTW-414 / GTW-415 / GTW-580: the gangs family's load coverage — the thin
//! wrapper over the generic per-family suite (`load_suite::suite`), plus the
//! family-bespoke roster-membership pin.
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! only binds it to [`GangsFamily`] with the authored gang stems. The
//! live-battle-start proof (the migrated skirmish + real gangs spawning the
//! concrete expected ganger set through `setup_battle`) is the bespoke
//! migration pin in `tests/load_gangs_spawn.rs` — split out to keep both files
//! within the repo file caps, never dropped.

mod load_suite;

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangerName};
use gdtf_content_families::GangsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a safety net against a genuine never-resolve
/// hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for GangsFamily {
    /// The shipped gang stems: `gang_0.gang.ron` / `gang_1.gang.ron`.
    const EXPECTED_MEMBERS: &'static [&'static str] = &["gang_0", "gang_1"];

    fn is_empty(registry: &GangRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &GangRegistry, label: &str) -> bool {
        registry.roster(&GangName::new(label.to_owned())).is_some()
    }
}

/// Tier (a) — the gangs loader registration + folder kick-off no-op cleanly
/// under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn gangs_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<GangsFamily>();
}

/// Tier (a) companion — the Load→Intro transition GATES on the
/// [`GangRegistry`] (the GTW-415 gate clause: the gangs folder is verified
/// loaded before any battle resolves a placed ganger's `(gang, member)` ref).
#[test]
fn load_does_not_leave_without_a_gang_registry() {
    suite::load_gates_on_registry::<GangsFamily>();
}

/// Tier (b) — the REAL `assets/content/gangs/` folder resolves into a
/// stem-keyed [`GangRegistry`] through the Load code path, with NO seeded
/// default registry (existence implies the real resolve ran).
#[test]
fn real_asset_resolves_gang_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<GangsFamily>();
}

/// Family-bespoke pin — the resolved rosters hold their MEMBERS: `gang_0`
/// carries "Alex Mercer" and `gang_1` carries both "Vex 1" and "Vex 2" (the
/// GTW-414 migration's roster shape, beyond the generic stem-resolves check).
/// Member names are content KEYS, not magnitudes, so this stays value-agnostic.
#[test]
fn real_asset_gang_rosters_hold_their_members() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async gangs folder load (cap is a safety net, GTW-305).
    advance_until_resource_exists::<GangRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<GangRegistry>();
    assert!(
        registry.is_some(),
        "the real gangs folder load must insert a GangRegistry within the safety-net budget",
    );
    let Some(registry) = registry else {
        return;
    };

    // gang_0 resolves and holds "Alex Mercer".
    let gang_0 = registry.roster(&GangName::new("gang_0".to_owned()));
    assert!(
        gang_0
            .and_then(|roster| roster.member(&GangerName::new("Alex Mercer".to_owned())))
            .is_some(),
        "the registry must hold gang_0 (keyed by gang_0.gang.ron's stem) with member \"Alex Mercer\"",
    );

    // gang_1 resolves and holds both "Vex 1" and "Vex 2".
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
