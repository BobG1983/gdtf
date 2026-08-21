use bevy::{
    app::App,
    asset::{AssetPlugin, uuid::Uuid},
    ecs::relationship::Relationship,
    prelude::{Entity, Messages, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, MovementOccurred, movement::WalkInProgress},
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverHp, HeightBand},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Position, Speed, Strength, Toughness},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    prelude::{Faction, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    terrain::{
        def::{TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainUuid},
        emplacement::{EmplacementState, MountedBy, MountedWeaponEntity},
        entity::TerrainCell,
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
    test_support::{
        GangerSpawnBuilder, TEST_MOUNTED_WEAPON_KEY, single_mode, test_armor_registry,
        test_melee_weapon_registry, test_terrain_registry, test_weapon_spec,
    },
    tuning::{
        CombatTuning, ExitEmplacementTu, ReactionCapBase, ReactionCapPerReactions, ReactionPMax,
        ReactionPMin, ReactionTuning, SuppressionRadius, SuppressionStabilityPenalty, ViewRange,
    },
    weapon::{
        BaseSpread, DamageType, FatalBias, FireMode, Kickback, MountedWeapon, WeaponName,
        WeaponRegistry, WeaponSpec, WieldedBy,
    },
};

pub(crate) const PLAYER: u8 = 0;

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
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            gun_spec(MOUNT_DAMAGE_TYPE),
        ),
    ])
}

/// Reaction tuning whose interrupt is certain rather than a roll: `p_min == p_max == 1.0`.
pub(crate) const fn forced_reaction_tuning(cap: u32) -> ReactionTuning {
    ReactionTuning {
        cap_base:            ReactionCapBase::new(cap as f32),
        cap_per_reactions:   ReactionCapPerReactions::new(0.0),
        p_min:               ReactionPMin::new(1.0),
        p_max:               ReactionPMax::new(1.0),
        suppression_radius:  SuppressionRadius::new(0),
        suppression_penalty: SuppressionStabilityPenalty::new(0.0),
    }
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
    app.insert_resource(test_terrain_registry());
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

/// The one emplacement seeded at `at`, or a failure naming the cell.
pub(crate) fn seated_emplacement(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TerrainCell, &EmplacementState)>();
    let matches: Vec<Entity> = query
        .iter(world)
        .filter(|(_, cell, _)| ***cell == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exactly one emplacement must be seeded at {at:?}, found {}",
        matches.len(),
    );
    let [entity] = matches[..] else {
        unreachable!("the count above is one");
    };
    entity
}

pub(crate) fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

pub(crate) fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedBy>(entity).map(Relationship::get)
}

/// Where a ganger stands right now.
pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|at| **at)
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

/// Rewrite the exit-emplacement leaf, which [`battle_app`] otherwise leaves at its default.
pub(crate) fn set_exit_tu(app: &mut App, tu: u8) {
    let world = app.world_mut();
    let Some(mut tuning) = world.get_resource_mut::<CombatTuning>() else {
        unreachable!("battle_app inserts the combat tuning this harness reads back");
    };
    tuning.exit_emplacement_tu = ExitEmplacementTu::new(tu);
}

/// The one player ganger standing on `at`, or a failure naming the cell and the count found.
pub(crate) fn player_on(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, faction, position)| ***faction == PLAYER && ***position == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one player ganger must stand on {at:?}, found {}",
        found.len(),
    );
    let [entity] = found[..] else {
        unreachable!("the count above is one");
    };
    entity
}

/// Enter the seat, and fail unless the enter actually manned it.
pub(crate) fn mount(app: &mut App, actor: Entity, emplacement: Entity) {
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(app, 3);
    assert_eq!(
        state(app, emplacement),
        Some(EmplacementState::Occupied),
        "PRECONDITION: the enter must man the seat, or nothing below is about a mounted ganger",
    );
    assert_eq!(
        occupant(app, emplacement),
        Some(actor),
        "PRECONDITION: the seat must name this actor as its occupant — a failed enter leaves \
         MountedBy absent and every assertion below passes while proving nothing",
    );
}

/// Whether this actor is still walking a committed route.
pub(crate) fn is_walking(app: &App, actor: Entity) -> bool {
    app.world().get::<WalkInProgress>(actor).is_some()
}

/// Every `MovementOccurred` written since the buffer was last drained.
pub(crate) fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

/// The one-sided emplacement def, whose single authored side makes a constrained route visible.
pub(crate) const ONE_SIDED: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_1186_0001));

/// The only side [`one_sided_emplacement`] authors, before a placed facing turns it.
pub(crate) const AUTHORED_SIDE: TerrainFacing = TerrainFacing::North;

/// The facing that piece is seeded at, which turns its one authored side to East.
pub(crate) const PLACED_FACING: TerrainFacing = TerrainFacing::East;

/// An emplacement def naming exactly one entry side, so one rotated entry cell reaches it.
pub(crate) fn one_sided_emplacement() -> TerrainDef {
    TerrainDef {
        key:            ONE_SIDED,
        display_name:   TerrainDisplayName::new("One-Sided Mount".to_owned()),
        sim_kind:       TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            entry_sides:      vec![AUTHORED_SIDE],
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("emplacement".to_owned()),
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}
