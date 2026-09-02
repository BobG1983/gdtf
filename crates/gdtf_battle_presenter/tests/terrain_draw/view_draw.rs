//! A piece draws the view its def, its facing and its open state select.

use bevy::{
    app::App,
    ecs::{entity::Entity, message::Messages},
    math::URect,
};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    openable::OpenState,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
    terrain::facing::TerrainFacing,
    test_support::test_pieces,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::harness::*;

// The keys `test_door()` and `test_facing_wall()` author for the rows these cases read.
const DOOR_SHUT_NORTH: &str = "door";
const DOOR_OPEN_NORTH: &str = "door_ns";
const DOOR_OPEN_EAST: &str = "door_ew";
const WALL_EDGE_NORTH: &str = "wall";
const WALL_EDGE_EAST: &str = "wall_ew";

fn empty_battle(app: &mut App) {
    insert_occupancy(app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);
}

fn battle_ready(app: &mut App) {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();
}

fn open_the_door(app: &mut App, door: Entity) {
    let state = app.world_mut().get_mut::<OpenState>(door);
    assert!(
        state.is_some(),
        "the door piece must carry an OpenState to flip",
    );
    if let Some(mut state) = state {
        *state = OpenState::Open;
    }
}

// Both views resolve to a rect and the two differ, or two `None` reads compare equal.
fn distinct_views(
    defs: &SpriteDefRegistry,
    one: &str,
    other: &str,
) -> (Option<URect>, Option<URect>) {
    let first = def_rect(defs, one);
    let second = def_rect(defs, other);
    assert!(
        first.is_some() && second.is_some(),
        "`{one}` and `{other}` must both resolve to a sheet rect, or every rect comparison \
         below is `None == None` — got {first:?} and {second:?}",
    );
    assert_ne!(
        first, second,
        "`{one}` and `{other}` must draw different rects, or the view this case pins cannot be \
         told from the other one",
    );
    (first, second)
}

fn spawn_door(app: &mut App, at: CellLevel, facing: TerrainFacing) -> Entity {
    let door = spawn_terrain_entity(app, at, test_pieces::DOOR, facing, None);
    app.world_mut().entity_mut(door).insert(OpenState::Closed);
    door
}

#[test]
fn a_door_draws_a_different_sprite_open_and_shut() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    empty_battle(&mut app);
    let door = spawn_door(&mut app, cell, TerrainFacing::North);
    battle_ready(&mut app);

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (shut, open) = distinct_views(&defs, DOOR_SHUT_NORTH, DOOR_OPEN_NORTH);

    assert_eq!(
        sprite_rect_at(&mut app, cell),
        shut,
        "a shut door draws its def's `Shut(North)` view",
    );

    open_the_door(&mut app, door);
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, cell),
        open,
        "an open door draws its def's `Open(North)` view — returning the shut view for both \
         states leaves this rect equal to the shut rect",
    );
}

#[test]
fn a_doors_view_survives_a_band_change_redraw() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    empty_battle(&mut app);
    let door = spawn_door(&mut app, cell, TerrainFacing::North);
    battle_ready(&mut app);

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (_shut, open) = distinct_views(&defs, DOOR_SHUT_NORTH, DOOR_OPEN_NORTH);

    open_the_door(&mut app, door);
    app.update();

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, cell),
        open,
        "raising the active level despawns and respawns every tile with no OpenState change on \
         that frame, so restamping only on a row's own change leaves the door on its shut view",
    );
}

#[test]
fn a_doors_view_survives_a_battle_ready_redraw() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    empty_battle(&mut app);
    let door = spawn_door(&mut app, cell, TerrainFacing::North);
    battle_ready(&mut app);

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (_shut, open) = distinct_views(&defs, DOOR_SHUT_NORTH, DOOR_OPEN_NORTH);

    open_the_door(&mut app, door);
    app.update();

    // A full redraw with no piece component changed on that frame.
    battle_ready(&mut app);

    assert_eq!(
        sprite_rect_at(&mut app, cell),
        open,
        "a battle-ready redraw restamps every tile from its piece, and the door's resolved view \
         is read afresh, so a restamp that ignores the respawn leaves the shut rect here",
    );
}

#[test]
fn one_door_def_at_two_facings_draws_two_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let north_cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    let east_cell = CellLevel::new(Cell::new(8, 6), Level::new(0));
    empty_battle(&mut app);
    let north = spawn_door(&mut app, north_cell, TerrainFacing::North);
    let east = spawn_door(&mut app, east_cell, TerrainFacing::East);
    battle_ready(&mut app);

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (open_north, open_east) = distinct_views(&defs, DOOR_OPEN_NORTH, DOOR_OPEN_EAST);

    open_the_door(&mut app, north);
    open_the_door(&mut app, east);
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, north_cell),
        open_north,
        "the north-facing door draws its def's `Open(North)` view",
    );
    assert_eq!(
        sprite_rect_at(&mut app, east_cell),
        open_east,
        "the east-facing door draws its def's `Open(East)` view — both doors are open, so a \
         resolver that drops the facing stamps the same view on both cells",
    );
}

#[test]
fn one_wall_def_at_two_facings_draws_two_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let north_cell = CellLevel::new(Cell::new(3, 3), Level::new(0));
    let east_cell = CellLevel::new(Cell::new(5, 3), Level::new(0));
    empty_battle(&mut app);
    spawn_terrain_entity(
        &mut app,
        north_cell,
        test_pieces::FACING_WALL,
        TerrainFacing::North,
        None,
    );
    spawn_terrain_entity(
        &mut app,
        east_cell,
        test_pieces::FACING_WALL,
        TerrainFacing::East,
        None,
    );
    battle_ready(&mut app);

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (edge_north, edge_east) = distinct_views(&defs, WALL_EDGE_NORTH, WALL_EDGE_EAST);

    assert_eq!(
        sprite_rect_at(&mut app, north_cell),
        edge_north,
        "the north-facing wall draws its def's `Edge(North)` view",
    );
    assert_eq!(
        sprite_rect_at(&mut app, east_cell),
        edge_east,
        "the east-facing wall draws its def's `Edge(East)` view — dropping the facing stamps \
         one view on both cells, and picking a Corner view stamps a third rect that is \
         neither",
    );
}
