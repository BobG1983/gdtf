use bevy::{
    app::App,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness},
    prelude::{Cell, CellLevel, Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, single_mode, test_armor_registry, test_melee_weapon_spec,
        test_terrain_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, FISTS_KEY, FireMode, Kickback, MeleeWeaponRegistry, MeleeWeaponSpec,
        Shove, WeaponName, WeaponPunch, WeaponRegistry, WeaponSpec,
    },
};

pub(crate) const SEED: u64 = 0x5E1E_5405;
pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;
pub(crate) const TEST_VIEW_RANGE: u16 = 12;

pub(crate) const fn level0() -> gdtf_battle_sim::metric::Level {
    gdtf_battle_sim::metric::Level::new(0)
}

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

pub(crate) fn melee_spec(shove: bool) -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        punch: WeaponPunch::new(9),
        shove: Shove::new(shove),
        ..test_melee_weapon_spec()
    }
}

pub(crate) fn ranged_spec(shove: bool) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.01),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        punch: WeaponPunch::new(20),
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        shove: Shove::new(shove),
        ..test_weapon_spec()
    }
}

pub(crate) fn melee_registry(shove: bool) -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([(WeaponName::new(FISTS_KEY.to_owned()), melee_spec(shove))])
}

pub(crate) fn ranged_registry(shove: bool) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new("test-weapon".to_owned()),
        ranged_spec(shove),
    )])
}

pub(crate) fn battle_app(melee_shove: bool, ranged_shove: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(ranged_registry(ranged_shove));
    app.insert_resource(melee_registry(melee_shove));
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app
}

pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
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

pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> u8 {
    *app.world().get::<Tu>(entity).copied().unwrap_or(Tu::new(0))
}

pub(crate) fn set_tu(app: &mut App, entity: Entity, pool: u8) {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(entity) {
        *tu = Tu::new(pool);
    }
}

pub(crate) fn tuning_of(app: &App) -> CombatTuning {
    app.world()
        .get_resource::<CombatTuning>()
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn strong_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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

pub(crate) fn defenceless_target(at: CellLevel, faction: u8) -> GangerSpawn {
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

pub(crate) fn fighting_target(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(120.0))
        .build()
}

pub(crate) fn weak_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        .build()
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
