//! A smashed floor drops the ganger on it only when its def leaves no slab standing.

use bevy::{app::App, asset::uuid::Uuid, prelude::Entity};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    battle::SetupBattleRequested,
    entity::{TerrainCell, TerrainIndex, TerrainIndexKey, TerrainPieceKind},
    ganger::{Aim, Direction, Facing, GangRegistry, Grit, Position, Speed, Toughness},
    injuries::{InjuryRegistry, InjuryTables},
    metric::{Cell, CellLevel, Level},
    occupancy_sync::TerrainPieceDestroyed,
    rng::{BattleSeed, InjuryRng, SeverityRng},
    situation::{GangerSpawn, Situation},
    slab::SlabHp,
    surface::{SlabState, SurfaceGrid},
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        piece::TerrainGraphicKey,
    },
    test_support::{
        GangerSpawnBuilder, SimAppBuilder, SituationBuilder, single_mode, test_terrain_registry,
    },
};

/// A one-hit floor that leaves nothing behind.
const HOLLOW_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0101));
/// A one-hit floor that leaves [`SUCCESSOR_SLAB`] behind.
const REPLACED_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0102));
/// The floor a smashed [`REPLACED_SLAB`] leaves standing.
const SUCCESSOR_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0103));

const SEED: u64 = 0x0523_1308;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The floor the faller stands on, one storey up from the shooter.
fn floor_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(1))
}

fn slab_def(key: TerrainUuid, name: &str, hp: u32, leaves_behind: LeavesBehind) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(name.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(hp),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        views: TerrainViews::new(Vec::new()),
        tags: Vec::new(),
        on_death: Vec::new(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind,
    }
}

fn falls_registry() -> TerrainDefRegistry {
    let mut registry = test_terrain_registry();
    registry.insert(
        HOLLOW_SLAB,
        slab_def(HOLLOW_SLAB, "Hollow Slab", 1, LeavesBehind::Nothing),
    );
    registry.insert(
        REPLACED_SLAB,
        slab_def(
            REPLACED_SLAB,
            "Replaced Slab",
            1,
            LeavesBehind::Piece(SUCCESSOR_SLAB),
        ),
    );
    registry.insert(
        SUCCESSOR_SLAB,
        slab_def(SUCCESSOR_SLAB, "Successor Slab", 80, LeavesBehind::Nothing),
    );
    registry
}

/// A live battle with a shooter below, a faller above, and the named floor between them.
fn battle_on(piece: TerrainUuid) -> (App, Entity, Entity) {
    let mut app = SimAppBuilder::new()
        .with_seed(SEED)
        .with_battle()
        .with_registries()
        .build();
    app.insert_resource(falls_registry());
    // `apply_falls` reads these five and no-ops without them, so the fall would never resolve.
    app.insert_resource(InjuryTables::default());
    app.insert_resource(InjuryRegistry::default());
    app.insert_resource(SeverityRng::from_root(BattleSeed::new(SEED)));
    app.insert_resource(InjuryRng::from_root(BattleSeed::new(SEED)));

    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([sharp_shooter(ground(5, 5)), sharp_shooter(floor_cell())])
        .slab_piece_at(floor_cell(), piece)
        .build_with_gangs();
    drive_setup(&mut app, situation, gangs);

    let shooter = ganger_on(&mut app, ground(5, 5));
    let faller = ganger_on(&mut app, floor_cell());
    (app, shooter, faller)
}

fn sharp_shooter(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .facing(Facing::new(Direction::East))
        .speed(Speed::new(20.0))
        .aim(Aim::new(20.0))
        .grit(Grit::new(20.0))
        .toughness(Toughness::new(30.0))
        .build()
}

fn drive_setup(app: &mut App, situation: Situation, gangs: GangRegistry) {
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

fn ganger_on(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    let found = query
        .iter(world)
        .find(|(_, position)| ***position == at)
        .map(|(entity, _)| entity);
    let Some(entity) = found else {
        unreachable!("setup spawns a ganger at every authored cell")
    };
    entity
}

fn indexed_slab(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<TerrainIndex>()
        .and_then(|index| index.get(&TerrainIndexKey::Slab(floor_cell())))
}

fn slab_state(app: &App) -> Option<SlabState> {
    app.world()
        .get_resource::<SurfaceGrid>()
        .map(|surface| surface.slab_state(&floor_cell()))
}

fn level_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Position>(entity).map(|at| *at.level())
}

fn piece_key_at_floor(app: &mut App) -> Option<TerrainUuid> {
    let world = app.world_mut();
    let mut query = world.query::<(&TerrainCell, &TerrainUuid)>();
    query
        .iter(world)
        .find(|(cell, _)| ***cell == floor_cell())
        .map(|(_, uuid)| *uuid)
}

/// Fire up at the floor until the piece the index names is no longer the one that stood there.
fn fire_until_replaced(app: &mut App, shooter: Entity) {
    let before = indexed_slab(app);
    assert!(
        before.is_some(),
        "the floor must be indexed before the firing loop, or the loop exits without a shot",
    );
    for _ in 0..64 {
        if indexed_slab(app) != before {
            return;
        }
        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            floor_cell().cell(),
            floor_cell().level(),
        ));
        for _ in 0..4 {
            app.update();
        }
    }
}

fn settle_destruction(app: &mut App) {
    app.world_mut().write_message(TerrainPieceDestroyed::new(
        floor_cell(),
        TerrainPieceKind::Slab,
    ));
    app.update();
    app.update();
}

#[test]
fn a_fired_floor_that_leaves_nothing_behind_drops_the_ganger_standing_on_it() {
    let (mut app, shooter, faller) = battle_on(HOLLOW_SLAB);
    assert_eq!(
        level_of(&app, faller),
        Some(1),
        "the faller starts upstairs"
    );

    fire_until_replaced(&mut app, shooter);

    assert_eq!(
        slab_state(&app),
        Some(SlabState::Absent),
        "a smashed floor whose def leaves nothing behind reads Absent",
    );
    assert_eq!(
        level_of(&app, faller),
        Some(0),
        "the hole drops the ganger to the level below",
    );
}

#[test]
fn a_fired_floor_that_leaves_a_slab_behind_holds_the_ganger_up() {
    let (mut app, shooter, faller) = battle_on(REPLACED_SLAB);

    fire_until_replaced(&mut app, shooter);

    assert_eq!(
        piece_key_at_floor(&mut app),
        Some(SUCCESSOR_SLAB),
        "the entity at the floor cell carries the successor slab def's own key",
    );
    assert_eq!(
        slab_state(&app),
        Some(SlabState::Present),
        "the successor slab stands, so the cell reads Present",
    );
    assert_eq!(
        level_of(&app, faller),
        Some(1),
        "nothing opened under the ganger, so its level is unchanged",
    );
}

#[test]
fn a_written_message_for_a_floor_that_leaves_nothing_behind_drops_the_ganger() {
    let (mut app, _shooter, faller) = battle_on(HOLLOW_SLAB);

    settle_destruction(&mut app);

    assert_eq!(
        slab_state(&app),
        Some(SlabState::Absent),
        "a smashed floor whose def leaves nothing behind reads Absent",
    );
    assert_eq!(
        level_of(&app, faller),
        Some(0),
        "the hole drops the ganger to the level below on the written path too",
    );
}

#[test]
fn a_written_message_for_a_floor_that_leaves_a_slab_behind_holds_the_ganger_up() {
    let (mut app, _shooter, faller) = battle_on(REPLACED_SLAB);

    settle_destruction(&mut app);

    assert_eq!(
        piece_key_at_floor(&mut app),
        Some(SUCCESSOR_SLAB),
        "the entity at the floor cell carries the successor slab def's own key",
    );
    assert_eq!(
        slab_state(&app),
        Some(SlabState::Present),
        "the successor slab stands, so the cell reads Present",
    );
    assert_eq!(
        level_of(&app, faller),
        Some(1),
        "nothing opened under the ganger on the written path either",
    );
}
