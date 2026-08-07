use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Aim, Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, TEST_WEAPON_KEY, test_armor_registry, test_melee_weapon_registry,
        test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, FireModeSpec, HitType,
        Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, TrajectoryStyle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponSpec,
    },
};

pub(crate) const PLAYER: u8 = 0;

pub(crate) const TEST_VIEW_RANGE: u16 = 30;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn at_level(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

pub(crate) fn grenade_spec(radius: u8) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        accuracy: Accuracy::new(0.8),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(2.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Blast,
        magazine: Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        fire_mode: FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.35),
            ModeShots::new(1),
            HitType::Blast {
                radius: BlastRadius::new(radius),
            },
        )]),
        trajectory: TrajectoryStyle::Arc,
        ..test_weapon_spec()
    }
}

pub(crate) fn grenade_registry(radius: u8) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        grenade_spec(radius),
    )])
}

pub(crate) fn battle_app(seed: u64, radius: u8) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(grenade_registry(radius));
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

pub(crate) fn thrower(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .aim(Aim::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

pub(crate) fn target(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(30.0))
        .cool(Cool::new(30.0))
        .toughness(Toughness::new(30.0))
        .build()
}

pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
