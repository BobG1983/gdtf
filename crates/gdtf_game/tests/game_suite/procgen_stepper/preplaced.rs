use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_sim::{
    battle::SetupBattleRequested, situation::PlacedGanger, test_support::fixtures,
};
use gdtf_game::test_support::PreplacedGangers;

use super::harness::{FIXED_SEED, app_engaged_in_generation, drive_stepper_to_done};

// The placements every request written so far carried, taken off the queue.
fn drain_placements(app: &mut App) -> Vec<Vec<PlacedGanger>> {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .drain()
        .map(|request| request.placements)
        .collect()
}

#[test]
fn a_stepper_engaged_generation_carries_preplaced_gangers() {
    let (_situation, settled) = fixtures::two_ganger();
    let mut app = app_engaged_in_generation(FIXED_SEED);
    app.world_mut()
        .insert_resource(PreplacedGangers::new(settled.clone()));

    drive_stepper_to_done(&mut app);

    assert_eq!(
        drain_placements(&mut app),
        vec![settled],
        "PreplacedGangers must replace the generated deployment on the stepped route too, so a \
         caller that settled the cells itself gets those cells spawned",
    );
}
