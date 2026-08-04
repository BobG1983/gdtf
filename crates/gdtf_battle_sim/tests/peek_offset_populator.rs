//! HARNESS NOTE — a wall, in the live sim, lives in BOTH grids: the `OccupancyGrid` as
use bevy::{
    app::App,
    math::Vec2,
    prelude::{IntoScheduleConfigs, MinimalPlugins},
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Facing,
    los::{Observer, PeekOffset, Target, has_los, has_los_peeking},
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, StairEyeOffset, TerrainKind},
    occupancy_sync::{CoverDestroyed, sync_destroyed_cover},
    peek_sync::{peek_population_needed, sync_peek_offsets},
    prelude::{Direction, Faction, Position, Stance, StanceKind},
    surface::SurfaceGrid,
    test_support::GangerEntityBuilder,
    tuning::CombatTuning,
};

fn key(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn ganger_cell() -> CellLevel {
    key(3, 3)
}

const fn expected_lean() -> PeekOffset {
    PeekOffset::new(Vec2::new(0.0, -0.4))
}

fn corner_grid(terrain: TerrainKind) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    grid.set_terrain(key(4, 3), terrain);
    grid.set_terrain(key(4, 4), terrain);
    grid
}

fn corner_cover() -> CoverLedger {
    let mut cover = CoverLedger::new();
    for at in [key(4, 3), key(4, 4)] {
        cover.insert(
            at,
            CoverEntry::seeded(
                CoverHp::new(50),
                HeightBand::High,
                ArmorProtection::new(5),
                ArmorHardness::new(2),
            ),
        );
    }
    cover
}

fn populator_app(grid: OccupancyGrid) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<CoverDestroyed>();
    app.insert_resource(grid);
    app.add_systems(
        bevy::prelude::Update,
        sync_peek_offsets.run_if(peek_population_needed),
    );
    app
}

fn peeking_ganger(app: &mut App, at: CellLevel) -> bevy::prelude::Entity {
    let ganger = GangerEntityBuilder::new()
        .at(at)
        .stance(StanceKind::Standing)
        .facing(Direction::North)
        .faction(Faction::new(0))
        .spawn(app.world_mut());
    app.world_mut()
        .entity_mut(ganger)
        .insert(PeekOffset::default());
    ganger
}

fn peek_of(app: &App, entity: bevy::prelude::Entity) -> Option<PeekOffset> {
    app.world().get::<PeekOffset>(entity).copied()
}

fn move_ganger(app: &mut App, entity: bevy::prelude::Entity, to: CellLevel) {
    if let Some(mut position) = app.world_mut().get_mut::<Position>(entity) {
        *position = Position::new(to);
    }
}

#[test]
fn populates_corner_peek_and_enables_around_corner_los() {
    let mut app = populator_app(corner_grid(TerrainKind::Wall));
    let entity = peeking_ganger(&mut app, ganger_cell());

    app.update();

    let Some(peek) = peek_of(&app, entity) else {
        unreachable!("the spawned ganger carries PeekOffset");
    };
    assert_eq!(
        peek,
        expected_lean(),
        "a ganger at the E-wall corner (open South end) must be auto-leaned South (0, -0.4)",
    );

    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = corner_cover();
    let occupancy = corner_grid(TerrainKind::Wall);

    let obs_pos = Position::new(ganger_cell());
    let obs_stance = Stance::new(StanceKind::Standing);
    let obs_facing = Facing::new(Direction::North);
    let tgt_pos = Position::new(key(5, 2));
    let tgt_stance = Stance::new(StanceKind::Standing);
    let target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };
    let centred = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };

    let centred_sighted = has_los(
        &centred,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        |_| false,
    );
    assert!(
        !*centred_sighted,
        "the CENTRED eye must be BLOCKED by the HIGH corner cover (the control)",
    );

    let peeked_sighted = has_los_peeking(
        &centred,
        &target,
        peek,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        |_| false,
    );
    assert!(
        *peeked_sighted,
        "the AUTO-POPULATED peek must let the ganger SEE the target around the corner",
    );
    assert_ne!(
        *centred_sighted, *peeked_sighted,
        "the populated peek must CHANGE the verdict (the producer→consumer bridge)",
    );
}

#[test]
fn moving_away_clears_the_peek() {
    let mut app = populator_app(corner_grid(TerrainKind::Wall));
    let entity = peeking_ganger(&mut app, ganger_cell());
    app.update();
    assert_eq!(
        peek_of(&app, entity),
        Some(expected_lean()),
        "precondition: the corner ganger has the South lean",
    );

    move_ganger(&mut app, entity, key(10, 10));
    app.update();

    assert_eq!(
        peek_of(&app, entity),
        Some(PeekOffset::default()),
        "after moving to open ground the peek must CLEAR to default (no stale peek)",
    );
}

#[test]
fn open_ground_gets_no_peek() {
    let mut app = populator_app(corner_grid(TerrainKind::Wall));
    let entity = peeking_ganger(&mut app, key(20, 20));
    app.update();

    assert_eq!(
        peek_of(&app, entity),
        Some(PeekOffset::default()),
        "a ganger on open ground hugs no corner → PeekOffset stays default",
    );
}

#[test]
fn cover_destroyed_clears_a_stationary_peek() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<CoverDestroyed>();
    app.insert_resource(corner_grid(TerrainKind::Cover));
    app.add_systems(
        bevy::prelude::Update,
        (
            sync_destroyed_cover,
            sync_peek_offsets.run_if(peek_population_needed),
        )
            .chain(),
    );
    let entity = peeking_ganger(&mut app, ganger_cell());
    app.update();
    assert_eq!(
        peek_of(&app, entity),
        Some(expected_lean()),
        "precondition: the cover-corner ganger has the South lean",
    );

    app.world_mut()
        .write_message(CoverDestroyed::new(key(4, 3)));
    app.update();

    assert_eq!(
        peek_of(&app, entity),
        Some(PeekOffset::default()),
        "destroying the corner cover must re-evaluate the STATIONARY ganger and clear its peek",
    );
}
