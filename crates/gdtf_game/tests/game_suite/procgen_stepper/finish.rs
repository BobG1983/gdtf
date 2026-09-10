use bevy::{app::App, ecs::message::Messages};
use cobalt_test_utils::advance_until;
use gdtf_battle_sim::{
    battle::{BattleInProgress, SetupBattleRequested},
    procgen::{ProcgenAdvance, StagedProcgen},
};
use gdtf_game::test_support::{
    AutoRunning, AutoStepTimer, BattleGenerationContext, BattleScapeState, PendingStepCommand,
    StepCommand,
};

use super::harness::{
    FIXED_SEED, app_engaged_in_generation, battlescape_state, drive_stepper_to_done,
};

/// Frames the held generation is given to prove it writes nothing.
const HOLD_FRAMES: u32 = 16;

fn requests(app: &App) -> usize {
    app.world()
        .get_resource::<Messages<SetupBattleRequested>>()
        .map_or(0, Messages::len)
}

// Every generation-owned resource still in the world, named so a failure says which survived.
fn surviving_resources(app: &App) -> Vec<&'static str> {
    let world = app.world();
    let present = [
        (
            "StagedProcgen",
            world.get_resource::<StagedProcgen>().is_some(),
        ),
        (
            "BattleGenerationContext",
            world.get_resource::<BattleGenerationContext>().is_some(),
        ),
        (
            "ProcgenAdvance",
            world.get_resource::<ProcgenAdvance>().is_some(),
        ),
        (
            "PendingStepCommand",
            world.get_resource::<PendingStepCommand>().is_some(),
        ),
        ("AutoRunning", world.get_resource::<AutoRunning>().is_some()),
        (
            "AutoStepTimer",
            world.get_resource::<AutoStepTimer>().is_some(),
        ),
    ];
    present
        .into_iter()
        .filter_map(|(name, alive)| alive.then_some(name))
        .collect()
}

#[test]
fn a_part_way_generation_writes_no_setup_request() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
    assert!(
        pending.is_some(),
        "PendingStepCommand must exist while the stepper is engaged in Generation",
    );
    if let Some(mut pending) = pending {
        pending.request(StepCommand::Next);
    }
    app.update();

    for frame in 0..HOLD_FRAMES {
        app.update();
        assert_eq!(
            requests(&app),
            0,
            "a driver that is part-way through its stages must write NO battle-setup request; \
             frame {frame} found one",
        );
    }

    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "and no battle may have been set up behind it",
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "the machine stays in Generation while the driver is held",
    );
}

#[test]
fn a_finished_generation_deploys_the_roster_it_emitted() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    drive_stepper_to_done(&mut app);

    let carried: Vec<usize> = app
        .world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .drain()
        .map(|request| request.placements.len())
        .collect();
    assert!(
        matches!(carried.as_slice(), [count] if *count > 0),
        "a driver that emitted a level must deploy the authored roster over it, so the one \
         request carries a non-empty placement list; got {carried:?}",
    );
}

#[test]
fn leaving_generation_leaves_no_generation_resource_behind() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    drive_stepper_to_done(&mut app);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let surviving = surviving_resources(&app);
    assert!(
        surviving.is_empty(),
        "nothing one generation inserts may outlive Generation, or the next battle starts \
         under the last one's rules; these survived: {surviving:?}",
    );
}
