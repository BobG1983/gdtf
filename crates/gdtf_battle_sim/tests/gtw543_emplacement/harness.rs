//! Shared GTW-543 emplacement fixture: the two-gun registries, the live battle-app driver,
//! the emplacement spawner, and the shared state / TU accessors.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    BaseSpread, Cool, DamageType, Faction, FatalBias, FireMode, Grit, Kickback, Speed, Stance,
    StanceKind, Strength, Toughness, Tu,
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
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
    weapon::{MountedWeapon, WeaponName, WeaponRegistry, WeaponSpec, WieldedBy},
};

/// Gang `0` is the player.
pub(crate) const PLAYER: u8 = 0;

/// The mounted-weapon registry key the test emplacement references.
pub(crate) const MOUNTED_KEY: &str = "test-mounted";

/// The ganger's OWN carried ranged weapon key (from `test_ranged_registry`) — its `DamageType` is
/// distinct from the mount's, so a `ShotFired`'s `DamageType` discriminates which gun fired.
pub(crate) const OWN_KEY: &str = "test-own-gun";

/// A view range covering the adjacent scene (arbitrary test tuning).
pub(crate) const TEST_VIEW_RANGE: u16 = 12;

/// The mounted gun's `DamageType` — DISTINCT from the ganger's own gun (below), so a `ShotFired`'s
/// `damage` field tells which weapon `dispatch_fire` resolved.
pub(crate) const MOUNT_DAMAGE_TYPE: DamageType = DamageType::Plasma;

/// The ganger's own carried gun's `DamageType` — distinct from the mount's.
pub(crate) const OWN_DAMAGE_TYPE: DamageType = DamageType::Las;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A single-shot `WeaponSpec` with the given `DamageType` (arbitrary, non-pinned handling numbers).
/// The two guns in the test differ ONLY in their `DamageType`, so a fired `ShotFired`'s `damage`
/// discriminates the mount from the ganger's own gun.
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

/// A `WeaponRegistry` holding BOTH the ganger's own carried gun ([`OWN_KEY`]) and the emplacement's
/// mounted gun ([`MOUNTED_KEY`]) — the two differ only in `DamageType`. Stands in for the app's
/// `Load`-built registry (the emplacement's `MountedWeaponKey` resolves `MOUNTED_KEY` against it).
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

/// Build the FULL live-runtime harness (the gtw508 `battle_app` idiom): `MinimalPlugins` +
/// `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin`, a `CombatTuning` (view range + default
/// emplacement TU leaves), and the persistent `Load` registries (with the two-gun ranged registry).
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

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle it.
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

/// The entity of the (sole) player ganger.
pub(crate) fn player_ganger(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)
}

/// A full-stat player ganger carrying the OWN-gun key so its own ranged weapon resolves at setup.
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

/// Spawn a VACANT emplacement terrain entity at `at` carrying the enter/exit state components +
/// the mounted-weapon key + a seeded (armorless) `CoverLedger` entry — exactly what `setup_battle`
/// attaches for an `Emplacement` piece. Returns its `Entity`.
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

/// An intact armorless cover entry (the emplacement's structural HP; armor irrelevant to this test).
pub(crate) const fn cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(45),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
}

/// The emplacement's current [`EmplacementState`].
pub(crate) fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

/// The emplacement's recorded occupant, if any.
pub(crate) fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<EmplacementOccupant>(entity).map(|o| **o)
}

/// The emplacement's recorded mounted-weapon entity, if any.
pub(crate) fn mount_entity(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedWeaponEntity>(entity).map(|m| **m)
}

/// The grid's occupant band at `at`.
pub(crate) fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

/// The current TU of `entity`.
pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The shipped `enter_emplacement_tu` leaf (read from the tuning resource, never hard-coded).
pub(crate) fn enter_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.enter_emplacement_tu)
}

/// The shipped `exit_emplacement_tu` leaf (read from the tuning resource, never hard-coded).
pub(crate) fn exit_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.exit_emplacement_tu)
}

/// Whether `ganger` currently wields a [`MountedWeapon`]-marked entity — resolved by scanning the
/// [`MountedWeapon`]-marked weapon entities for one whose [`WieldedBy`] back-reference points at
/// `ganger`. `true` only while manning an emplacement (the mount is spawned + related on enter,
/// despawned on exit).
pub(crate) fn wields_mount(app: &mut App, ganger: Entity) -> bool {
    use bevy::ecs::relationship::Relationship;
    let world = app.world_mut();
    let mut query = world.query_filtered::<&WieldedBy, bevy::prelude::With<MountedWeapon>>();
    query
        .iter(world)
        .any(|wielded_by| wielded_by.get() == ganger)
}

/// Step `app` a fixed number of ticks so a written message dispatches + settles.
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
