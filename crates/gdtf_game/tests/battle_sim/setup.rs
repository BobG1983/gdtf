use bevy::state::state::State;
use gdtf_battle_sim::{
    armor::Wears,
    cover::CoverLedger,
    occupancy::OccupancyGrid,
    rng::ShotRng,
    situation::Situation,
    surface::SurfaceGrid,
    test_support::{SituationBuilder, ganger_at, key},
    tuning::CombatTuning,
    vertical::{LinkKind, VerticalLink, VerticalLinkGraph},
};
use gdtf_game::test_support::{BattleRunningComplete, BattleScapeState, GameState};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// Frames the machine is given to prove it stays put — per-frame work, no IO.
const HOLD_FRAMES: u32 = 96;

fn dangling_link_situation() -> Situation {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1);
    SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present)
        .vertical_link(VerticalLink::new(present, missing, LinkKind::stair()))
        .build()
}

#[test]
fn setup_battle_lands_resources_and_spawns_gangers() {
    let situation = two_ganger_situation();
    let authored_gangers = situation.gangers.len();
    let mut app = walk_app(Some(situation));
    drive_to_generation(&mut app);

    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "setup_battle must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "setup_battle must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "setup_battle must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "setup_battle must insert a VerticalLinkGraph",
    );

    app.update();
    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        authored_gangers,
        "the spawned ganger count (each wearing armor via Wears) must equal the authored count",
    );
}

#[test]
fn failed_setup_does_not_advance_generation() {
    let mut app = walk_app(Some(dangling_link_situation()));
    drive_to_generation(&mut app);

    for _ in 0..HOLD_FRAMES {
        app.update();
    }
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "the machine must remain in Generation when setup failed",
    );

    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "a failed setup must insert no CoverLedger (it aborts before any resource insert)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "a failed setup must insert no OccupancyGrid",
    );
}

#[test]
fn battle_resources_survive_battle_and_clean_on_exit() {
    let mut app = walk_app(Some(two_ganger_situation()));
    drive_to_generation(&mut app);

    advance_until(&mut app, |app| {
        matches!(
            battlescape_state(app),
            Some(BattleScapeState::AnimateIn | BattleScapeState::BattleRunning)
        )
    });
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "RNG streams must survive past Generation (battle-lifetime)",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "CoverLedger must survive past Generation",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "SurfaceGrid must survive past Generation",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "OccupancyGrid must survive past Generation",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "VerticalLinkGraph must survive past Generation",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning (E10.4's persistent Load resource) must still be present",
    );

    app.world_mut().insert_resource(BattleRunningComplete);
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<State<GameState>>()
            .is_none_or(|state| *state.get() != GameState::BattleScape)
    });
    assert!(
        app.world().get_resource::<ShotRng>().is_none(),
        "RNG streams must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "CoverLedger must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_none(),
        "SurfaceGrid must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "OccupancyGrid must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_none(),
        "VerticalLinkGraph must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must NOT be removed by this plugin (it is the persistent Load resource)",
    );
}

#[test]
fn empty_situation_builds_and_advances() {
    let mut app = walk_app(None);
    drive_to_generation(&mut app);

    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "the empty Default situation must still insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "the empty Default situation must still insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the empty Default situation must still insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "the empty Default situation must still insert a VerticalLinkGraph",
    );

    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        0,
        "the empty Default situation spawns zero gangers",
    );

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::AnimateIn)
    });
}
