use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::MeleeResolved,
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness},
    metric::{Cell, CellLevel, Level},
    occupancy_sync::CoverDestroyed,
    prelude::{Faction, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

pub(crate) const PLAYER: u8 = 0;

pub(crate) const TEST_VIEW_RANGE: u16 = 12;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

pub(crate) fn drive_setup(
    app: &mut App,
    seed: u64,
    situation_and_gangs: (Situation, GangRegistry),
) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn player_ganger(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

pub(crate) fn cover_hp(app: &App, at: CellLevel) -> Option<u32> {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .map(|entry| *entry.current_hp)
}

pub(crate) fn cover_entry_at(app: &App, at: CellLevel) -> Option<CoverEntry> {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .copied()
}

pub(crate) fn cover_destroyed_flag(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .is_some_and(|entry| *entry.destroyed)
}

#[derive(Resource, Default)]
pub(crate) struct MeleeLog {
    hits: Vec<MeleeResolved>,
}

#[derive(Resource, Default)]
pub(crate) struct DestroyedLog {
    hits: Vec<CoverDestroyed>,
}

pub(crate) fn record_melee(
    mut resolved: bevy::prelude::MessageReader<MeleeResolved>,
    mut log: bevy::prelude::ResMut<MeleeLog>,
) {
    for hit in resolved.read() {
        log.hits.push(*hit);
    }
}

pub(crate) fn record_destroyed(
    mut destroyed: bevy::prelude::MessageReader<CoverDestroyed>,
    mut log: bevy::prelude::ResMut<DestroyedLog>,
) {
    for hit in destroyed.read() {
        log.hits.push(*hit);
    }
}

pub(crate) fn with_logs(app: &mut App) {
    app.init_resource::<MeleeLog>();
    app.init_resource::<DestroyedLog>();
    app.add_systems(bevy::app::Update, (record_melee, record_destroyed));
}

pub(crate) fn melee_hits(app: &App) -> usize {
    app.world()
        .get_resource::<MeleeLog>()
        .map_or(0, |log| log.hits.len())
}

pub(crate) fn destroyed_hits(app: &App) -> usize {
    app.world()
        .get_resource::<DestroyedLog>()
        .map_or(0, |log| log.hits.len())
}

pub(crate) fn attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

pub(crate) fn seed_cover(app: &mut App, at: CellLevel, max_hp: u32) {
    let entry = CoverEntry::seeded(
        CoverHp::new(max_hp),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    );
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, entry);
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
