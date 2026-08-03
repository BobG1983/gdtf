use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{
        Aim, Aiming, Cool, Direction, Facing, GangRegistry, Grit, Hp, Speed, Strength, Toughness,
        Wounds,
    },
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
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, HitType, Kickback,
        ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage, WeaponName,
        WeaponPunch, WeaponRegistry, WeaponSpec,
    },
};

pub(crate) const PLAYER: u8 = 0;

pub(crate) const TEST_VIEW_RANGE: u16 = 20;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn aoe_weapon_spec(hit_type: HitType) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(2.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Blast,
        fire_mode: FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
            hit_type,
        )]),
        stable: Stable::new(true),
        ..test_weapon_spec()
    }
}

pub(crate) fn aoe_registry(hit_type: HitType) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        aoe_weapon_spec(hit_type),
    )])
}

pub(crate) fn battle_app(seed: u64, hit_type: HitType) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(aoe_registry(hit_type));
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

pub(crate) fn shooter(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .aiming(Aiming::new(true))
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

pub(crate) fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

pub(crate) fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

pub(crate) const fn fire_mode(hit_type: HitType) -> FireModeSpec {
    FireModeSpec::with_hit_type(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
        hit_type,
    )
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

pub(crate) fn is_fielded(app: &App, entity: Entity) -> bool {
    app.world().get::<Hp>(entity).is_some()
}

pub(crate) fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (hp_of(app, entity), wounds_of(app, entity))
}

pub(crate) const fn took_damage(
    before: (Option<u16>, Option<u8>),
    after: (Option<u16>, Option<u8>),
) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}
