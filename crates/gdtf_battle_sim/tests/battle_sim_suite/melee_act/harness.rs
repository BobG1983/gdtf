use bevy::{
    app::App,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    acts::MeleeResolved,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Hp, Speed, Strength, Toughness, Wounds},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, LifeState, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::{PlacedGanger, Situation},
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

use super::connect::{BrokenLog, StruckLog, record_broken, record_struck};

pub(crate) const SEED: u64 = 0x5E1E_9907;

pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

pub(crate) const TEST_VIEW_RANGE: u16 = 12;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn battle_app_with_tuning(tuning: CombatTuning) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(tuning);
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

pub(crate) fn battle_app() -> App {
    battle_app_with_tuning(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    })
}

pub(crate) fn drive_setup(
    app: &mut App,
    situation_and_gangs: (Situation, Vec<PlacedGanger>, GangRegistry),
) {
    let (situation, placements, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        placements,
        BattleSeed::new(SEED),
    ));
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

pub(crate) fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

pub(crate) fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

pub(crate) fn life_of(app: &App, entity: Entity) -> Option<LifeState> {
    app.world().get::<LifeState>(entity).copied()
}

#[derive(Resource, Default)]
pub(crate) struct MeleeLog {
    hits: Vec<MeleeResolved>,
}

pub(crate) fn record_melee(
    mut resolved: bevy::prelude::MessageReader<MeleeResolved>,
    mut log: bevy::prelude::ResMut<MeleeLog>,
) {
    for hit in resolved.read() {
        log.hits.push(*hit);
    }
}

pub(crate) fn with_melee_log(app: &mut App) {
    app.init_resource::<MeleeLog>();
    app.init_resource::<StruckLog>();
    app.init_resource::<BrokenLog>();
    app.add_systems(
        bevy::app::Update,
        (record_melee, record_struck, record_broken),
    );
}

pub(crate) fn melee_hits(app: &App) -> usize {
    app.world()
        .get_resource::<MeleeLog>()
        .map_or(0, |log| log.hits.len())
}

pub(crate) fn strong_attacker(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .build()
}

pub(crate) fn defenceless_target(
    at: CellLevel,
    faction: u8,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        .toughness(Toughness::new(12.0))
        .build()
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
