use bevy::state::state::State;
use gdtf_battle_sim::{
    acts::FireRequested,
    cover::{CoverLedger, HeightBand},
    ganger::{Hp, LifeState, Wounds},
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    rng::ShotRng,
    surface::SurfaceGrid,
    test_support::{key, single_mode},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};
use gdtf_game::test_support::{BattleScapeState, GameState};

use super::harness::*;

const AUTHORED_GANGER_COUNT: usize = 2;

fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

#[test]
fn bootstrap_reaches_battlescape_with_the_sim_constructed() {
    let mut app = bootstrap_app();

    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape after reaching BattleRunning",
    );

    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "Generation's setup_battle must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "Generation's setup_battle must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "Generation's setup_battle must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "Generation's setup_battle must insert a VerticalLinkGraph",
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "Generation must insert the seeded RNG streams",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must be present in BattleRunning",
    );

    let world = app.world_mut();
    let mut worn = world.query::<&gdtf_battle_sim::armor::Wears>();
    assert_eq!(
        worn.iter(world).count(),
        AUTHORED_GANGER_COUNT,
        "the world must hold exactly the authored ganger count of gangers wearing armor (Wears) — \
         the real setup_battle spawned the gangers, not a no-op scaffold",
    );
}

#[test]
fn the_drive_is_panic_free_and_seed_deterministic_across_runs() {
    let post_fire_target_state = || -> Option<(Option<Hp>, Option<Wounds>, Option<LifeState>)> {
        let mut app = bootstrap_app();
        let shooter = find_ganger(&mut app, SHOOTER_FACTION)?;
        let target = find_ganger(&mut app, TARGET_FACTION)?;

        let mode = single_mode(0.2, 1);
        app.world_mut()
            .entity_mut(shooter)
            .insert(shooter_weapon_kit(mode));
        let (tx, ty, tl) = TARGET_AT;
        let target_at = key(tx, ty, tl);
        if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
            grid.set_occupant_band(target_at, Some(HeightBand::High));
        }

        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(tx, ty),
            Level::new(tl),
        ));
        app.update();

        Some((
            app.world().get::<Hp>(target).copied(),
            app.world().get::<Wounds>(target).copied(),
            app.world().get::<LifeState>(target).copied(),
        ))
    };

    let first = post_fire_target_state();
    let second = post_fire_target_state();
    assert!(
        first.is_some(),
        "the seeded drive must reach BattleRunning and fire",
    );
    assert_eq!(
        first, second,
        "the post-fire (Hp, Wounds, LifeState) tuple must be identical across two fixed-seed runs \
         (got {first:?} vs {second:?}) — the same BattleSeed source reproduces the same outcome",
    );
}

#[test]
fn drive_is_headless_and_stops_at_battle_running() {
    let app = bootstrap_app();

    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the headless drive must stop at BattleScapeState::BattleRunning, never advancing into \
         AfterMath",
    );
}
