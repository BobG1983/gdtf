//! bleed / DOT / field kills, so `resolve_on_death` can fan its authored on-death effect.
//! HARNESS NOTE: this drives the REAL [`apply_falls`] system on the same bespoke falls harness
use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, ResMut, Resource},
};
use gdtf_battle_sim::{
    effects::on_death::OnDeathOccurred,
    falls::FallsPlugin,
    ganger::{Hp, Luck, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    occupancy_sync::{OccupancyMaintenancePlugin, SlabDestroyed},
    prelude::{
        Cell, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Stance, StanceKind, Tu,
    },
    surface::SurfaceGrid,
    test_support::SimAppBuilder,
    tuning::{CombatTuning, PerStoreyDamage},
};

const SEED: u64 = 0x0547_FA11_DEAD_BEEF;
const COL_X: i32 = 5;
const COL_Y: i32 = 5;

const fn column_cell() -> Cell {
    Cell::new(COL_X, COL_Y)
}

#[derive(Resource, Default)]
struct DeathLog {
        deaths: Vec<OnDeathOccurred>,
}

fn record_deaths(mut reader: MessageReader<OnDeathOccurred>, mut log: ResMut<DeathLog>) {
    for death in reader.read() {
        log.deaths.push(*death);
    }
}

fn falls_app(seed: u64, per_storey: PerStoreyDamage) -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(seed)
        .with_acts()
        .with_player_faction(1)
        .with_tuning(CombatTuning {
            per_storey_damage: per_storey,
            ..Default::default()
        })
        .build();
    app.add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    app.init_resource::<DeathLog>();
    app.add_systems(Update, record_deaths);
    app
}

fn spawn_frail_faller(app: &mut App, level: u8) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(CellLevel::new(column_cell(), Level::new(level))),
            Stance::new(StanceKind::Standing),
            Faction::new(1),
            Tu::new(100),
            TuMax::new(100),
            Hp::new(4),
            Wounds::new(1),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(0.0),
            Luck::new(0.0),
        ))
        .id()
}

fn destroy_slab_and_settle(app: &mut App, level: u8) {
    app.world_mut()
        .write_message(SlabDestroyed::new(CellLevel::new(
            column_cell(),
            Level::new(level),
        )));
    app.update();
    app.update();
}

fn life_of(app: &App, entity: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(entity)
        .copied()
        .unwrap_or(LifeState::Alive)
}

fn death_emitted_for(app: &App, entity: Entity) -> bool {
    app.world()
        .get_resource::<DeathLog>()
        .is_some_and(|log| log.deaths.iter().any(|d| d.entity == entity))
}

#[test]
fn a_ganger_killed_by_a_fall_emits_on_death() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(1000));
    let faller = spawn_frail_faller(&mut app, 2);
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 2);

    assert_eq!(
        life_of(&app, faller),
        LifeState::Dead,
        "a lethal multi-storey fall kills the frail faller"
    );
    assert!(
        death_emitted_for(&app, faller),
        "apply_falls emits OnDeathOccurred for a fall-killed faller (the falls terminal-death \
         gate) so resolve_on_death can fan its on-death effect"
    );
}
