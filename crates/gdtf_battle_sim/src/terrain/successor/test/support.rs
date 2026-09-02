use bevy::{app::App, asset::uuid::Uuid, prelude::Entity};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    battle::SetupBattleRequested,
    cover::{CoverHp, HeightBand},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::{CoverSpawn, Situation, SlabSpawn},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag, TerrainUuid, TerrainViews,
        },
        entity::{TerrainBrace, TerrainCell, TerrainPieceKind},
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
        vertical::{LinkKind, VerticalLink},
    },
    test_support::{SimAppBuilder, SituationBuilder, TEST_SEED, ganger_at, test_terrain_registry},
};

/// A cover piece that leaves [`SUCCESSOR_WALL`] behind.
pub(super) const SMASHED_COVER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0001));
/// A blocking wall, spawned as a successor.
pub(super) const SUCCESSOR_WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0002));
/// A cover piece that leaves nothing behind.
pub(super) const PLAIN_COVER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0003));
/// The slab the cover cases stand on.
pub(super) const PLAIN_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0004));
/// A slab that leaves [`SUCCESSOR_SLAB`] behind.
pub(super) const BRACED_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0005));
/// The slab spawned in a braced slab's place.
pub(super) const SUCCESSOR_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1308_0006));

pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

fn display(name: &str) -> TerrainDisplayName {
    TerrainDisplayName::new(name.to_owned())
}

fn graphic(role: &str) -> TerrainGraphicKey {
    TerrainGraphicKey::new(role.to_owned())
}

fn cover_def(key: TerrainUuid, name: &str, leaves_behind: LeavesBehind) -> TerrainDef {
    TerrainDef {
        key,
        display_name: display(name),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: graphic("cover"),
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind,
    }
}

fn wall_def(key: TerrainUuid, name: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: display(name),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(80),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: graphic("wall"),
        },
        tags: vec![TerrainTag::BlocksPathfinding],
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn slab_def(key: TerrainUuid, name: &str, leaves_behind: LeavesBehind) -> TerrainDef {
    TerrainDef {
        key,
        display_name: display(name),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(60),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: graphic("floor"),
            footfall:     None,
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind,
    }
}

/// The test registry plus the six defs these cases author.
pub(super) fn successor_registry() -> TerrainDefRegistry {
    let mut registry = test_terrain_registry();
    registry.insert(
        SMASHED_COVER,
        cover_def(
            SMASHED_COVER,
            "Smashed Cover",
            LeavesBehind::Piece(SUCCESSOR_WALL),
        ),
    );
    registry.insert(SUCCESSOR_WALL, wall_def(SUCCESSOR_WALL, "Successor Wall"));
    registry.insert(
        PLAIN_COVER,
        cover_def(PLAIN_COVER, "Plain Cover", LeavesBehind::Nothing),
    );
    registry.insert(
        PLAIN_SLAB,
        slab_def(PLAIN_SLAB, "Plain Slab", LeavesBehind::Nothing),
    );
    registry.insert(
        BRACED_SLAB,
        slab_def(
            BRACED_SLAB,
            "Braced Slab",
            LeavesBehind::Piece(SUCCESSOR_SLAB),
        ),
    );
    registry.insert(
        SUCCESSOR_SLAB,
        slab_def(SUCCESSOR_SLAB, "Successor Slab", LeavesBehind::Nothing),
    );
    registry
}

/// A live battle holding the named cover pieces, and a slab under each.
pub(super) fn battle_with(covers: &[(CellLevel, TerrainUuid)]) -> App {
    let mut app = SimAppBuilder::new().with_battle().with_registries().build();
    app.insert_resource(successor_registry());

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    for (at, piece) in covers {
        situation
            .scatter
            .push(CoverSpawn::new(*at, *piece, TerrainFacing::East));
        situation
            .slabs
            .push(SlabSpawn::new(*at, PLAIN_SLAB, TerrainFacing::North));
    }
    drive_setup(&mut app, situation, gangs);
    app
}

/// A live battle whose [`BRACED_SLAB`] at `at` stands over a stair endpoint at `below`.
pub(super) fn battle_with_braced_slab(below: CellLevel, at: CellLevel) -> App {
    let mut app = SimAppBuilder::new().with_battle().with_registries().build();
    app.insert_resource(successor_registry());

    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_piece_at(below, PLAIN_SLAB)
        .slab_piece_at(at, BRACED_SLAB)
        .vertical_link(VerticalLink::new(below, at, LinkKind::stair()))
        .build_with_gangs();
    drive_setup(&mut app, situation, gangs);
    app
}

fn drive_setup(app: &mut App, situation: Situation, gangs: crate::ganger::GangRegistry) {
    app.world_mut().insert_resource(gangs);
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        BattleSeed::new(TEST_SEED),
    ));
    for _ in 0..4 {
        app.update();
    }
}

/// Every entity that carries both this cell and this def key.
pub(super) fn pieces_keyed(app: &mut App, at: CellLevel, piece: TerrainUuid) -> Vec<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TerrainCell, &TerrainUuid)>();
    query
        .iter(world)
        .filter(|(_, cell, uuid)| ***cell == at && **uuid == piece)
        .map(|(entity, ..)| entity)
        .collect()
}

/// Whether the piece at this cell carrying this def key has terrain brace.
pub(super) fn carries_brace(app: &mut App, at: CellLevel, piece: TerrainUuid) -> bool {
    let standing = pieces_keyed(app, at, piece);
    standing
        .iter()
        .any(|entity| app.world().get::<TerrainBrace>(*entity).is_some())
}

/// The kind a destroyed-piece message names for a scatter cover piece.
pub(super) const COVER_KIND: TerrainPieceKind = TerrainPieceKind::Cover;

/// The kind a destroyed-piece message names for a slab.
pub(super) const SLAB_KIND: TerrainPieceKind = TerrainPieceKind::Slab;
