use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, ResMut, Resource, World},
};
use gdtf_battle_sim::{
    acts::ShoveRequested,
    falls::{FallOccurred, FallsPlugin},
    ganger::{Hp, Wounds},
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{Cell, CellLevel, Faction, Level, Position, StanceKind, Tu},
    test_support::{GangerEntityBuilder, SimAppBuilder},
    tuning::CombatTuning,
};

pub(crate) const SEED: u64 = 0x0525_5405_DEAD_BEEF;

pub(crate) fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}


#[derive(Resource, Default)]
pub(crate) struct FallLog {
        pub(crate) falls: Vec<FallOccurred>,
}

pub(crate) fn record_falls(mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>) {
    for signal in reader.read() {
        log.falls.push(*signal);
    }
}

pub(crate) fn shove_app() -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(SEED)
        .with_acts()
        .with_tuning(shipped_tuning())
        .build();
    app.add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    app.init_resource::<FallLog>();
    app.add_systems(Update, record_falls);
    app
}

pub(crate) fn shove_ganger(world: &mut World, at: CellLevel, faction: u8) -> Entity {
    GangerEntityBuilder::new()
        .at(at)
        .stance(StanceKind::Standing)
        .faction(Faction::new(faction))
        .tu(100)
        .tu_max(100)
        .combat_vitals(1000, 200)
        .toughness(0.0)
        .luck(0.0)
        .spawn(world)
}

pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

pub(crate) fn hp_of(app: &App, entity: Entity) -> u16 {
    *app.world().get::<Hp>(entity).copied().unwrap_or(Hp::new(0))
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> u8 {
    *app.world().get::<Tu>(entity).copied().unwrap_or(Tu::new(0))
}

pub(crate) fn wounds_of(app: &App, entity: Entity) -> u8 {
    *app.world()
        .get::<Wounds>(entity)
        .copied()
        .unwrap_or(Wounds::new(0))
}

pub(crate) fn shove_and_settle(app: &mut App, shover: Entity, target: Entity) {
    app.world_mut()
        .write_message(ShoveRequested::new(shover, target));
    app.update();
    app.update();
}

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn upper(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}
