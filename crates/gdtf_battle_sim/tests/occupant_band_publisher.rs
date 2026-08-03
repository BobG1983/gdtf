use bevy::{
    app::App,
    math::Vec3,
    prelude::{Entity, MinimalPlugins},
};
use gdtf_battle_sim::{
    cover::{CoverLedger, HeightBand},
    march::{MarchDir, MarchKind, march_vector},
    metric::cell_center,
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Level, LifeState, OccupancyGrid, Position, SimPos, Stance, StanceKind,
    },
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

fn start_cell() -> CellLevel {
    CellLevel::new(Cell::new(10, 5), Level::new(0))
}

fn moved_cell() -> CellLevel {
    CellLevel::new(Cell::new(10, 8), Level::new(0))
}

fn publisher_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

fn spawn_target(app: &mut App, at: CellLevel, stance: StanceKind) -> Entity {
    app.world_mut()
        .spawn((Position::new(at), Stance::new(stance), LifeState::Alive))
        .id()
}

fn march_mid_round_through(app: &App, cell: CellLevel) -> MarchKind {
    let tuning = CombatTuning::default();
    let Some(occupancy) = app.world().get_resource::<OccupancyGrid>() else {
        return MarchKind::Miss;
    };
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    let mid_frac = f32::midpoint(
        *tuning.projectile_band_edges.low_mid,
        *tuning.projectile_band_edges.mid_high,
    );
    let center = cell_center(cell.cell(), Level::new(0));
    let muzzle = SimPos::new(center.x - 4.0, center.y, mid_frac);
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let shooter_cell = CellLevel::new(Cell::new(59, 59), Level::new(7));

    march_vector(
        muzzle,
        MarchDir::new(dir),
        occupancy,
        &surface,
        &cover,
        &tuning,
        shooter_cell,
        |_| false,
    )
    .kind
}

#[test]
fn move_plus_stance_change_publishes_current_band_at_new_cell() {
    let mut app = publisher_app();
    let target = spawn_target(&mut app, start_cell(), StanceKind::Standing);
    app.update(); 

    assert_eq!(
        march_mid_round_through(&app, start_cell()),
        MarchKind::Ganger(target),
        "initial standing placement: a MID round impacts the HIGH occupant",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(target) {
        *pos = Position::new(moved_cell());
    }
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(target) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();

    let mid_verdict = march_mid_round_through(&app, moved_cell());
    assert_ne!(
        mid_verdict,
        MarchKind::Ganger(target),
        "a MID round must sail OVER the now-PRONE (LOW) occupant at the new cell; a \
         stale STANDING-HIGH band would wrongly impact it (got {mid_verdict:?})",
    );

    let published = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&moved_cell()));
    assert_eq!(
        published,
        Some(HeightBand::Low),
        "the publisher wrote the CURRENT prone (LOW) band at the new cell (got {published:?})",
    );
}

#[test]
fn vacated_cell_reads_none_after_move() {
    let mut app = publisher_app();
    let target = spawn_target(&mut app, start_cell(), StanceKind::Standing);
    app.update(); 

    assert_eq!(
        march_mid_round_through(&app, start_cell()),
        MarchKind::Ganger(target),
        "pre-move: a MID round impacts the standing occupant at the start cell",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(target) {
        *pos = Position::new(moved_cell());
    }
    app.update();

    let verdict = march_mid_round_through(&app, start_cell());
    assert_ne!(
        verdict,
        MarchKind::Ganger(target),
        "the vacated cell must read None — a MID round through the OLD cell must NOT \
         impact the ganger that left it (got {verdict:?})",
    );
    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&start_cell()));
    assert_eq!(
        band, None,
        "the publisher cleared the vacated cell's band in lockstep with its occupant",
    );
}

#[test]
fn dead_occupant_cell_reads_none() {
    let mut app = publisher_app();
    let target = spawn_target(&mut app, start_cell(), StanceKind::Standing);
    app.update(); 

    assert_eq!(
        march_mid_round_through(&app, start_cell()),
        MarchKind::Ganger(target),
        "pre-death: a MID round impacts the live standing occupant",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(target) {
        *life = LifeState::Dead;
    }
    app.update();

    let verdict = march_mid_round_through(&app, start_cell());
    assert_ne!(
        verdict,
        MarchKind::Ganger(target),
        "on DEATH the occupied cell must read None — a MID round must NOT impact the \
         dead ganger's vacated cell (got {verdict:?})",
    );
    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&start_cell()));
    assert_eq!(
        band, None,
        "the publisher cleared the freed cell's band when the ganger died (GTW-459)",
    );
}
