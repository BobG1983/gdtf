//! The Generation state gate: plugin wiring, RNG stream seeding on entry, and
//! completion gated on setup.

use gdtf_app::test_support::BattleScapeState;
use gdtf_battle_sim::{
    battle::BattleInProgress,
    rng::{BattleSeed, ShotRng},
};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// AC1 — `BattleSimPlugin` adds `OccupancyMaintenancePlugin`: building the
/// battlescape plugin tree registers the `CoverDestroyed` message buffer (added
/// exactly once — no double-add panic), proving the reused maintenance layer is
/// wired and live.
#[test]
fn occupancy_maintenance_plugin_is_wired() {
    // Build the app (no need to drive — the plugin tree, and thus its message
    // registration, exists from construction).
    let app = walk_app(None);

    assert!(
        app.world()
            .get_resource::<bevy::ecs::message::Messages<gdtf_battle_sim::occupancy_sync::CoverDestroyed>>()
            .is_some(),
        "BattleSimPlugin must register the CoverDestroyed message buffer via \
         OccupancyMaintenancePlugin",
    );
}

/// AC2 — entering Generation seeds the five battle RNG streams (GTW-14), and the
/// seed is threaded through `ShotRng::from_root`. Two sub-properties:
///
/// (a) **Presence**: Generation inserts a `ShotRng` resource.
/// (b) **Seed sensitivity**: `ShotRng::from_root` of different seeds draws different
///     first values — the derivation `fnv1a64(seed_bytes ++ LABEL)` distinguishes seeds.
///
/// The exact seed the app chose (wall-clock or env-pinned) is not checked here —
/// that threading property is owned by the sim-level lifecycle tests
/// (`setup_threads_the_message_seed_through_rng_streams`). The app-level concern
/// is that Generation actually inserts the resource and that `from_root` is
/// seed-discriminating.
#[test]
fn entering_generation_seeds_rng_streams() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach BattleScapeState::Generation within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // (a) The stream resource is present after Generation is reached.
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "entering Generation must insert the ShotRng stream (GTW-14 five-stream setup)",
    );

    // (b) Seed sensitivity: two different seeds produce different first draws —
    // the derivation path (FNV-1a-64 + seed_from_u64 + ChaCha12) is sensitive to
    // the seed. This property holds without knowing which seed the app chose.
    let mut seed_zero = ShotRng::from_root(BattleSeed::new(0));
    let mut seed_one = ShotRng::from_root(BattleSeed::new(1));
    assert_ne!(
        seed_zero.next_u64(),
        seed_one.next_u64(),
        "different BattleSeeds must yield different ShotRng first draws",
    );
    // Reproducibility: the same seed produces the same first draw across two constructions.
    let mut a = ShotRng::from_root(BattleSeed::new(42));
    let mut b = ShotRng::from_root(BattleSeed::new(42));
    assert_eq!(
        a.next_u64(),
        b.next_u64(),
        "same BattleSeed must reproduce the same ShotRng draw",
    );
}

/// AC4 — Generation completion is GATED on real setup success: the state advances
/// to `AnimateIn`, and on the first update where the setup witness exists the gate
/// can fire — completion never precedes setup (the successful setup inserts the
/// `BattleInProgress` witness, GTW-212's explicit battle-active tag). Ordering
/// relation, not a frame count.
#[test]
fn generation_completion_is_gated_on_setup() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // Once Generation is reached the setup has succeeded, so the battle-active witness
    // exists; the gate (and thus move_on) then advances strictly after setup. Advancing
    // must reach AnimateIn, and the BattleInProgress witness must already be present (it
    // was inserted on the same successful-setup Ok path that signals BattleReady — the
    // sim-side band keys off it, GTW-212).
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a successful setup must have inserted the BattleInProgress witness before completion gates",
    );

    let reached_animate_in = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateIn),
        BUDGET,
    );
    assert!(
        reached_animate_in,
        "with setup succeeded, Generation must advance to AnimateIn within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}
