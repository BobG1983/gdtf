use gdtf_battle_sim::procgen::StagedProcgen;
use gdtf_game::test_support::{ProcgenStepperActive, ProcgenStepperPlugin};

use super::harness::{
    FIXED_SEED, app_ready_for_battle, drive_into_battle_running, terrain_fingerprint,
};

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
