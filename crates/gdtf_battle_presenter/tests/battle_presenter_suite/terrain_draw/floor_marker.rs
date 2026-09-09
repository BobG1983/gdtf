//! Where the magenta missing-tile marker shows once battle setup seeds the floor field.

use bevy::{app::App, asset::Assets, prelude::MeshMaterial2d};
use gdtf_battle_presenter::{MissingTileTexture, TerrainFogMaterial, TerrainSprite};
use gdtf_battle_sim::{
    def::TerrainUuid,
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    prelude::{Cell, CellLevel, Level},
    situation::{PlacedGanger, Situation},
    test_support::{SituationBuilder, ganger_at, key, test_pieces},
};

use super::harness::*;

// Inside the authored grid, and outside it but still inside the drawn extent.
const INSIDE: Cell = Cell::new(4, 4);
const OUTSIDE: Cell = Cell::new(20, 20);

fn ground(cell: Cell) -> CellLevel {
    CellLevel::new(cell, Level::new(0))
}

fn small_grid() -> GridSize {
    let size = GridSize::new(GridWidth::new(10), GridHeight::new(10), GridLevels::new(2));
    let Ok(size) = size else {
        unreachable!("10x10x2 is inside every grid-size bound");
    };
    size
}

fn situation_with_floor(default_floor: TerrainUuid) -> (Situation, Vec<PlacedGanger>) {
    SituationBuilder::new()
        .with_gangers([ganger_at(key(1, 1, 0), 0), ganger_at(key(2, 1, 0), 1)])
        .default_floor(default_floor)
        .grid_size(small_grid())
        .build()
}

fn marker_handle(app: &App) -> Option<bevy::asset::Handle<bevy::image::Image>> {
    app.world()
        .get_resource::<MissingTileTexture>()
        .map(MissingTileTexture::handle)
}

fn draws_the_marker(app: &mut App, at: CellLevel) -> Option<bool> {
    let missing = marker_handle(app)?;
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(tile, _)| tile.at == at)
        .map(|(_, mat)| mat.id())?;
    let image = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .image
        .id();
    Some(image == missing.id())
}

#[test]
fn a_cell_with_no_authored_piece_draws_the_situations_default_floor() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    drive_setup(&mut app, situation_with_floor(test_pieces::FLOOR));

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };

    assert_eq!(
        sprite_rect_at(&mut app, ground(INSIDE)),
        def_rect(&defs, "floor"),
        "a cell with no authored piece draws the default floor def's own sprite",
    );
    assert_eq!(
        draws_the_marker(&mut app, ground(INSIDE)),
        Some(false),
        "a cell the theme's default floor covers must NOT draw the missing-tile marker",
    );
}

#[test]
fn a_cell_outside_the_authored_grid_still_draws_the_default_floor() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    drive_setup(&mut app, situation_with_floor(test_pieces::FLOOR));

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };

    assert_eq!(
        sprite_rect_at(&mut app, ground(OUTSIDE)),
        def_rect(&defs, "floor"),
        "the presenter draws GRID_WIDTH by GRID_HEIGHT and takes no notice of the situation's \
         grid_size, so seeding only grid_size leaves this cell pieceless",
    );
    assert_eq!(
        draws_the_marker(&mut app, ground(OUTSIDE)),
        Some(false),
        "a drawn cell outside the authored grid must NOT draw the missing-tile marker",
    );
}

#[test]
fn a_situation_with_no_default_floor_draws_the_marker() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    drive_setup(&mut app, situation_with_floor(TerrainUuid::nil()));

    assert_eq!(
        draws_the_marker(&mut app, ground(INSIDE)),
        Some(true),
        "a storey-0 cell whose floor def does not resolve draws the magenta missing-tile \
         marker — that is the one authoring gap it signals",
    );
}
