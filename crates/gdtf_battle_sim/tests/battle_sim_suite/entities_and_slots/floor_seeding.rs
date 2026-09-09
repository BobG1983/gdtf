//! Battle setup seeds a floor piece on every storey-0 cell the presenter draws.

use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    def::TerrainUuid,
    entity::TerrainCell,
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::{Cell, CellLevel, Level},
    situation::{BattleRegistries, Situation, setup_battle},
    test_support::{
        SituationBuilder, ganger_at, test_armor_registry, test_melee_weapon_registry, test_pieces,
        test_terrain_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};

// Inside the authored grid, outside it but inside the drawn extent, and the authored wall.
const INSIDE: Cell = Cell::new(4, 4);
const OUTSIDE: Cell = Cell::new(20, 20);
const OVERRIDDEN: Cell = Cell::new(2, 3);
const WALLED: Cell = Cell::new(1, 2);

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

fn seeded_world() -> App {
    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([
            ganger_at(ground(Cell::new(1, 1)), 0),
            ganger_at(ground(Cell::new(2, 1)), 1),
        ])
        .default_floor(test_pieces::FLOOR)
        .floor_at(
            ground(OVERRIDDEN),
            test_pieces::SLAB,
            gdtf_battle_sim::terrain::facing::TerrainFacing::East,
        )
        .grid_size(small_grid())
        .wall_at(ground(WALLED))
        .build_with_gangs();
    run_setup(situation, gangs)
}

fn run_setup(situation: Situation, gangs: gdtf_battle_sim::ganger::GangRegistry) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    let weapons = test_weapon_registry();
    let melee = test_melee_weapon_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let terrain = test_terrain_registry();
    let fallback_floor_cost = CombatTuning::default().move_costs.open;
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            let done = setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &weapons,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                ),
                fallback_floor_cost,
                &mut commands,
            );
            done.is_ok()
        });
    assert_eq!(
        outcome.ok(),
        Some(true),
        "setup_battle must complete on this situation",
    );
    app
}

fn pieces_at(app: &mut App, at: CellLevel) -> Vec<(Entity, Option<TerrainUuid>)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TerrainCell, Option<&TerrainUuid>)>();
    query
        .iter(world)
        .filter(|(_, cell, _)| ***cell == at)
        .map(|(entity, _, piece)| (entity, piece.copied()))
        .collect()
}

fn one_piece_uuid(app: &mut App, at: CellLevel) -> Option<TerrainUuid> {
    let found = pieces_at(app, at);
    assert_eq!(
        found.len(),
        1,
        "exactly one terrain piece must stand at {at:?}, found {}",
        found.len(),
    );
    found.first().and_then(|(_, piece)| *piece)
}

#[test]
fn an_authored_floor_spawn_wins_over_the_situations_default_floor() {
    let mut app = seeded_world();
    assert_eq!(
        one_piece_uuid(&mut app, ground(OVERRIDDEN)),
        Some(test_pieces::SLAB),
        "a cell an authored FloorSpawn names carries THAT spawn's def, not the default floor",
    );
}

#[test]
fn a_cell_inside_the_grid_with_no_authoring_takes_the_default_floor() {
    let mut app = seeded_world();
    assert_eq!(
        one_piece_uuid(&mut app, ground(INSIDE)),
        Some(test_pieces::FLOOR),
        "a cell with neither a FloorSpawn nor authored terrain takes the default floor",
    );
}

#[test]
fn a_cell_outside_the_grid_but_inside_the_drawn_extent_takes_the_default_floor() {
    let mut app = seeded_world();
    assert_eq!(
        one_piece_uuid(&mut app, ground(OUTSIDE)),
        Some(test_pieces::FLOOR),
        "the presenter draws GRID_WIDTH by GRID_HEIGHT and never reads Situation::grid_size, \
         so seeding only grid_size leaves this drawn cell pieceless",
    );
}

#[test]
fn an_authored_wall_keeps_its_cell_to_itself() {
    let mut app = seeded_world();
    let found = pieces_at(&mut app, ground(WALLED));
    assert_eq!(
        found.len(),
        1,
        "exactly one entity carrying TerrainCell may stand at the wall's cell: two leave the \
         drawn graphic decided by query order — found {}",
        found.len(),
    );
    assert_eq!(
        found.first().and_then(|(_, piece)| *piece),
        Some(test_pieces::WALL),
        "the one piece at the wall's cell is the wall, not a floor seeded under it",
    );
}
