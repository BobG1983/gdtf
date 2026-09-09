use bevy::{app::App, prelude::Visibility};
use gdtf_battle_sim::{
    cover::CoverLedger,
    ganger::Facing,
    occupancy::StairEyeOffset,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, Stance,
        StanceKind,
    },
    surface::SurfaceGrid,
    test_support::SituationBuilder,
    tuning::CombatTuning,
    visibility::{FovObserver, union_fov},
};

use super::harness::*;

#[test]
fn terrain_renders_visible_explored_unseen() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let visible_cell = CellLevel::new(Cell::new(5, 5), l0);
    let explored_cell = CellLevel::new(Cell::new(6, 6), l0);
    let unseen_cell = CellLevel::new(Cell::new(7, 7), l0);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(visible_cell, 0, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, unseen_cell),
        "the terrain field must have drawn (every in-range cell is at least floor)",
    );

    set_fog(&mut app, &[visible_cell], &[explored_cell]);
    app.update();

    let visible_terrain = terrain_at(&mut app, visible_cell);
    assert!(
        visible_terrain.is_some(),
        "the VISIBLE cell must have a terrain tile + material"
    );
    let Some((vis_saturation, vis_flag)) = visible_terrain else {
        return;
    };
    assert!(
        (vis_saturation - 1.0).abs() < f32::EPSILON,
        "a squad-VISIBLE cell renders at full colour (saturation 1.0); got {vis_saturation}",
    );
    assert_eq!(vis_flag, Visibility::Inherited, "VISIBLE terrain is shown");

    let explored_terrain = terrain_at(&mut app, explored_cell);
    assert!(
        explored_terrain.is_some(),
        "the EXPLORED cell must have a terrain tile + material"
    );
    let Some((exp_saturation, exp_flag)) = explored_terrain else {
        return;
    };
    assert!(
        exp_saturation.abs() < f32::EPSILON,
        "an EXPLORED cell renders full-brightness GREYSCALE (saturation 0.0 — colour-loss as \
         the memory cue, not brightness-loss); got {exp_saturation}",
    );
    assert!(
        exp_saturation < vis_saturation,
        "EXPLORED is desaturated relative to VISIBLE (the memory cue is colour-loss)",
    );
    assert_eq!(
        exp_flag,
        Visibility::Inherited,
        "EXPLORED terrain is shown (greyscale, full brightness)",
    );

    let unseen_terrain = terrain_at(&mut app, unseen_cell);
    assert!(
        unseen_terrain.is_some(),
        "the UNSEEN cell must have a terrain tile + material"
    );
    let Some((_unseen_saturation, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "a never-seen cell does not render its terrain",
    );
}

fn dense_visible_from_observer(app: &App, at: CellLevel) -> Vec<CellLevel> {
    let occupancy = app.world().resource::<OccupancyGrid>().clone();
    let surface = app.world().resource::<SurfaceGrid>().clone();
    let cover = app.world().resource::<CoverLedger>().clone();
    let tuning = app.world().resource::<CombatTuning>().clone();
    let position = Position::new(at);
    let stance = Stance::new(StanceKind::Standing);
    let facing = Facing::new(Direction::East);
    let observers = [FovObserver {
        position:         &position,
        stance:           &stance,
        facing:           &facing,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, |_| false);
    visible.into_iter().collect()
}

#[test]
fn dense_floor_set_renders_lit_floor_around_observer() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let observer_cell = CellLevel::new(Cell::new(10, 10), l0);
    let near_floor = CellLevel::new(Cell::new(11, 10), l0);
    let far_floor = CellLevel::new(Cell::new(40, 40), l0);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(observer_cell, 0, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, far_floor),
        "the open-floor terrain field must have drawn (every in-range cell is at least floor)",
    );

    let visible = dense_visible_from_observer(&app, observer_cell);
    assert!(
        visible.contains(&observer_cell) && visible.contains(&near_floor),
        "the dense scan reveals the observer's open-floor cell + an in-range floor neighbour",
    );
    assert!(
        !visible.contains(&far_floor),
        "a floor cell beyond view_range is NOT in the dense VISIBLE set",
    );
    set_fog(&mut app, &visible, &[]);
    app.update();

    let observer_terrain = terrain_at(&mut app, observer_cell);
    let near_terrain = terrain_at(&mut app, near_floor);
    assert!(
        observer_terrain.is_some() && near_terrain.is_some(),
        "the observer + neighbour floor cells must have terrain sprites",
    );
    let (Some((obs_saturation, obs_flag)), Some((near_saturation, near_flag))) =
        (observer_terrain, near_terrain)
    else {
        return;
    };
    assert!(
        (obs_saturation - 1.0).abs() < f32::EPSILON,
        "the observer's open-floor cell renders at full colour (saturation 1.0 — lit, not black); \
         got {obs_saturation}",
    );
    assert_eq!(
        obs_flag,
        Visibility::Inherited,
        "the observer's open-floor cell is shown",
    );
    assert!(
        (near_saturation - 1.0).abs() < f32::EPSILON,
        "an in-range open-floor cell renders LIT at full colour (the dense fog reveals the \
         rendered floor); got {near_saturation}",
    );
    assert_eq!(
        near_flag,
        Visibility::Inherited,
        "an in-range open-floor cell is shown",
    );

    let far_terrain = terrain_at(&mut app, far_floor);
    assert!(
        far_terrain.is_some(),
        "the far open-floor cell must have a terrain tile (every cell is at least floor)",
    );
    let Some((_far_saturation, far_flag)) = far_terrain else {
        return;
    };
    assert_eq!(
        far_flag,
        Visibility::Hidden,
        "a floor cell beyond view_range stays Hidden — the fog horizon, not an all-black map",
    );
}
