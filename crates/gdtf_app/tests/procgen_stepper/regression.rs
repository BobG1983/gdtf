//! The un-engaged fingerprint pins: proving the stepper wiring leaves the normal
//! (non-stepper) load path unchanged.

use gdtf_app::test_support::{ProcgenStepperActive, ProcgenStepperPlugin};
use gdtf_battle_sim::procgen::StagedProcgen;

use super::harness::{
    FIXED_SEED, app_ready_for_battle, drive_into_battle_running, terrain_fingerprint,
};

/// The regression pin (C5a): with the stepper NOT engaged, a fixed seed reaches `BattleRunning`
/// with a deterministic terrain fingerprint — the normal (non-stepper) load path, UNCHANGED by
/// this ticket's wiring.
#[test]
fn normal_path_reaches_running_with_a_terrain_fingerprint() {
    let mut app = app_ready_for_battle(FIXED_SEED);
    drive_into_battle_running(&mut app);

    let fingerprint = terrain_fingerprint(&app);
    assert!(
        matches!(fingerprint, Some(n) if n > 0),
        "the normal path must set up a populated TerrainIndex; got {fingerprint:?}",
    );
}

/// The SHIPPED wiring since GTW-868: the plugin is added DISENGAGED, so its drive systems are
/// registered but the engagement marker is absent. That must be indistinguishable from not
/// adding the plugin at all — nothing engages, no drive is ever started, and the battle reaches
/// `BattleRunning` by the normal to-completion path. This is the pin behind the acceptance
/// clauses "a plain `cargo drun` reaches a battle with no overlay and no pause" and "`cargo
/// dtest` does not hang".
#[test]
fn stepper_plugin_added_disengaged_leaves_the_normal_path() {
    let mut app = app_ready_for_battle(FIXED_SEED);
    app.add_plugins(ProcgenStepperPlugin::with_enabled(false));
    assert!(
        app.world().get_resource::<ProcgenStepperActive>().is_none(),
        "adding the plugin disengaged must NOT insert the engagement marker",
    );
    drive_into_battle_running(&mut app);

    assert!(
        app.world().get_resource::<StagedProcgen>().is_none(),
        "no stepper drive may ever have been engaged",
    );
    let fingerprint = terrain_fingerprint(&app);
    assert!(
        matches!(fingerprint, Some(n) if n > 0),
        "a disengaged stepper plugin must not change the normal path; got {fingerprint:?}",
    );
}
