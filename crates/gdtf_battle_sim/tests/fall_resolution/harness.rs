use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, ResMut, Resource, World},
};
use gdtf_battle_sim::{
    acts::InjuryInflicted,
    falls::{FallOccurred, FallsPlugin},
    ganger::{Hp, Luck, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    occupancy_sync::{OccupancyMaintenancePlugin, SlabDestroyed},
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position, Stance, StanceKind, Tu},
    test_support::SimAppBuilder,
    tuning::{CombatTuning, PerStoreyDamage},
};

pub(crate) fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

pub(crate) const COL_X: i32 = 5;
pub(crate) const COL_Y: i32 = 5;

pub(crate) const fn column_cell() -> Cell {
    Cell::new(COL_X, COL_Y)
}

pub(crate) const SEED: u64 = 0x0523_FA11_DEAD_BEEF;


#[derive(Resource, Default)]
pub(crate) struct FallLog {
        falls: Vec<FallOccurred>,
}

#[derive(Resource, Default)]
pub(crate) struct InjuryLog {
        pub(crate) targets: Vec<Entity>,
}

pub(crate) fn record_falls(mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>) {
    for signal in reader.read() {
        log.falls.push(*signal);
    }
}

pub(crate) fn record_injuries(
    mut reader: MessageReader<InjuryInflicted>,
    mut log: ResMut<InjuryLog>,
) {
    for message in reader.read() {
        log.targets.push(message.target);
    }
}

pub(crate) fn falls_app(seed: u64, per_storey: PerStoreyDamage) -> App {
    let mut tuning = shipped_tuning();
    tuning.per_storey_damage = per_storey;
    let mut app = SimAppBuilder::new()
        .with_seed(seed)
        .with_acts()
        .with_player_faction(1)
        .with_tuning(tuning)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    app.init_resource::<FallLog>();
    app.init_resource::<InjuryLog>();
    app.add_systems(Update, (record_falls, record_injuries));
    app
}

pub(crate) fn spawn_faller(world: &mut World, level: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(column_cell(), Level::new(level))),
            Stance::new(StanceKind::Standing),
            Faction::new(1),
            Tu::new(100),
            TuMax::new(100),
            Hp::new(1000),
            Wounds::new(200),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(0.0),
            Luck::new(0.0),
        ))
        .id()
}

pub(crate) fn level_of(app: &App, entity: Entity) -> u8 {
    let pos = app
        .world()
        .get::<Position>(entity)
        .copied()
        .unwrap_or_else(Position::default);
    *pos.level()
}

pub(crate) fn hp_of(app: &App, entity: Entity) -> u16 {
    *app.world().get::<Hp>(entity).copied().unwrap_or(Hp::new(0))
}

pub(crate) fn destroy_slab_and_settle(app: &mut App, level: u8) {
    app.world_mut()
        .write_message(SlabDestroyed::new(CellLevel::new(
            column_cell(),
            Level::new(level),
        )));
    app.update();
    app.update();
}

pub(crate) fn fall_signals(app: &App) -> Vec<FallOccurred> {
    app.world()
        .get_resource::<FallLog>()
        .map(|log| log.falls.clone())
        .unwrap_or_default()
}
