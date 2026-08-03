use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    prelude::{Faction, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    terrain::{
        emplacement::{
            EmplacementOccupant, EmplacementState, MountedWeaponEntity, MountedWeaponKey,
        },
        entity::TerrainCell,
    },
    test_support::{
        GangerSpawnBuilder, single_mode, test_armor_registry, test_melee_weapon_registry,
        test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        BaseSpread, DamageType, FatalBias, FireMode, Kickback, MountedWeapon, WeaponName,
        WeaponRegistry, WeaponSpec, WieldedBy,
    },
};

pub(crate) const PLAYER: u8 = 0;

pub(crate) const MOUNTED_KEY: &str = "test-mounted";

pub(crate) const OWN_KEY: &str = "test-own-gun";

pub(crate) const TEST_VIEW_RANGE: u16 = 12;

pub(crate) const MOUNT_DAMAGE_TYPE: DamageType = DamageType::Plasma;

pub(crate) const OWN_DAMAGE_TYPE: DamageType = DamageType::Las;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn gun_spec(damage_type: DamageType) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        kickback: Kickback::new(0.2),
        fatal_bias: FatalBias::new(0.0),
        damage_type,
        fire_mode: FireMode::new(vec![single_mode(0.3, 1)]),
        ..test_weapon_spec()
    }
}

pub(crate) fn test_ranged_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            gun_spec(OWN_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(MOUNTED_KEY.to_owned()),
            gun_spec(MOUNT_DAMAGE_TYPE),
        ),
    ])
}

pub(crate) fn battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_ranged_registry());
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

pub(crate) fn player_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .weapon(WeaponName::new(OWN_KEY.to_owned()))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

pub(crate) fn spawn_emplacement(app: &mut App, at: CellLevel) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            TerrainCell::new(at),
            EmplacementState::Vacant,
            MountedWeaponKey::new(WeaponName::new(MOUNTED_KEY.to_owned())),
        ))
        .id();
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, cover_entry());
    entity
}

pub(crate) const fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(45),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
}

pub(crate) fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

pub(crate) fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<EmplacementOccupant>(entity).map(|o| **o)
}

pub(crate) fn mount_entity(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedWeaponEntity>(entity).map(|m| **m)
}

pub(crate) fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

pub(crate) fn enter_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.enter_emplacement_tu)
}

pub(crate) fn exit_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.exit_emplacement_tu)
}

pub(crate) fn wields_mount(app: &mut App, ganger: Entity) -> bool {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut query = world.query_filtered::<&WieldedBy, bevy::prelude::With<MountedWeapon>>();
    query
        .iter(world)
        .any(|wielded_by| wielded_by.get() == ganger)
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
