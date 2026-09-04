use gdtf_battle_sim::{
    battle::BattleInProgress,
    rng::{BattleSeed, ShotRng},
};
use gdtf_game::test_support::BattleScapeState;
use gdtf_test_utils::advance_until;

use super::harness::*;

#[test]
fn occupancy_maintenance_plugin_is_wired() {
    let app = walk_app(None);

    assert!(
        app.world()
            .get_resource::<bevy::ecs::message::Messages<gdtf_battle_sim::occupancy_sync::TerrainPieceDestroyed>>()
            .is_some(),
        "BattleSimPlugin must register the TerrainPieceDestroyed message buffer via \
         OccupancyMaintenancePlugin",
    );
}

#[test]
fn entering_generation_seeds_rng_streams() {
    let mut app = walk_app(Some(two_ganger_situation()));
    drive_to_generation(&mut app);

    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "entering Generation must insert the ShotRng stream",
    );

    let mut seed_zero = ShotRng::from_root(BattleSeed::new(0));
    let mut seed_one = ShotRng::from_root(BattleSeed::new(1));
    assert_ne!(
        seed_zero.next_u64(),
        seed_one.next_u64(),
        "different BattleSeeds must yield different ShotRng first draws",
    );
    let mut a = ShotRng::from_root(BattleSeed::new(42));
    let mut b = ShotRng::from_root(BattleSeed::new(42));
    assert_eq!(
        a.next_u64(),
        b.next_u64(),
        "same BattleSeed must reproduce the same ShotRng draw",
    );
}

#[test]
fn generation_completion_is_gated_on_setup() {
    let mut app = walk_app(Some(two_ganger_situation()));
    drive_to_generation(&mut app);

    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a successful setup must have inserted the BattleInProgress witness before completion gates",
    );

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::AnimateIn)
    });
}
