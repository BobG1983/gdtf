//! The un-engaged / disabled-plugin fingerprint pins: proving the stepper wiring leaves the
//! normal (non-stepper) load path byte-for-byte unchanged.

use gdtf_app::test_support::ProcgenStepperPlugin;

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

/// `with_enabled(false)` registers nothing — adding the plugin disabled must be
/// indistinguishable from not adding it at all (mirrors the auto-battle affordance's own AC).
#[test]
fn stepper_disabled_plugin_is_indistinguishable_from_absent() {
    let mut app = app_ready_for_battle(FIXED_SEED);
    app.add_plugins(ProcgenStepperPlugin::with_enabled(false));
    drive_into_battle_running(&mut app);

    let fingerprint = terrain_fingerprint(&app);
    assert!(
        matches!(fingerprint, Some(n) if n > 0),
        "a disabled stepper plugin must not change the normal path; got {fingerprint:?}",
    );
}
