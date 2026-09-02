//! Authoring the world a terrain-draw case reads: the grids, the pieces, the ledger entry.

use bevy::{
    app::App,
    ecs::{entity::Entity, message::Messages},
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::BattleReady,
    cover::{CoverEntry, CoverHp, HeightBand},
    def::TerrainUuid,
    entity::{TerrainCell, TerrainPieceKind},
    occupancy::{OccupancyInput, TerrainPlacement},
    piece::{FootfallSound, LeftoverSprite, TerrainGraphicKey},
    prelude::{CellLevel, OccupancyGrid},
    terrain::facing::TerrainFacing,
    test_support::test_pieces,
};

pub(crate) fn insert_occupancy(app: &mut App, terrain: Vec<TerrainPlacement>) {
    let input = OccupancyInput {
        terrain,
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(
        &input,
        &bevy::platform::collections::HashSet::default(),
    );
    app.world_mut().insert_resource(grid);
}

pub(crate) const fn low_cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(30),
        HeightBand::Low,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
        TerrainPieceKind::Cover,
    )
}

/// Spawn a piece the view resolver can read: its cell, its def uuid, the way it is turned,
/// and the surface it sounds like underfoot.
pub(crate) fn spawn_terrain_entity(
    app: &mut App,
    key: CellLevel,
    piece: TerrainUuid,
    facing: TerrainFacing,
    footfall: Option<&str>,
) -> Entity {
    let mut entity = app
        .world_mut()
        .spawn((TerrainCell::new(key), piece, facing));
    if let Some(footfall) = footfall {
        entity.insert(FootfallSound::new(footfall.to_owned()));
    }
    entity.id()
}

/// Stand the sprite a destroyed def leaves behind in a cell, the way the sim's successor
/// does: a cell and a sprite key, and no piece identity at all.
pub(crate) fn spawn_leftover_sprite(app: &mut App, key: CellLevel, sprite: &str) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(key),
            LeftoverSprite::new(TerrainGraphicKey::new(sprite.to_owned())),
        ))
        .id()
}

/// Despawn the terrain piece standing at a cell, which is what a destruction does.
pub(crate) fn despawn_terrain_entity(app: &mut App, key: CellLevel) {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TerrainCell)>();
    let standing: Vec<Entity> = query
        .iter(world)
        .filter(|(_, cell)| ***cell == key)
        .map(|(entity, _)| entity)
        .collect();
    for entity in standing {
        assert!(
            world.despawn(entity),
            "the terrain piece at {key:?} must still be alive to despawn",
        );
    }
}

pub(crate) const CENTER_WALL_DEF: &str = r#"(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",
        rect: (x: 0, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 8),
)"#;

pub(crate) fn draw_one_wall(app: &mut App, key: CellLevel) {
    insert_occupancy(
        app,
        vec![TerrainPlacement::new(
            key,
            gdtf_battle_sim::occupancy::TerrainKind::Wall,
        )],
    );
    app.world_mut()
        .insert_resource(gdtf_battle_sim::cover::CoverLedger::new());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::surface::SurfaceGrid::new());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::prelude::BattleInProgress);
    spawn_terrain_entity(app, key, test_pieces::WALL, TerrainFacing::North, None);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();
}
