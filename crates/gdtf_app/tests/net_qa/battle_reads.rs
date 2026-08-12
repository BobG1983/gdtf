//! Shared world reads the battle-read socket cases plan their requests from.

use bevy::{app::App, ecs::entity::Entity, platform::collections::HashSet};
use gdtf_app::qa_wire::{
    act_payload::{AimNet, FacingNet},
    cell::CellLevelNet,
    token::GangerToken,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    battle::PlayerFaction,
    ganger::{Aiming, Facing, Faction, LifeState, Position},
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
    vertical::VerticalLinkGraph,
};

/// The first player-faction ganger the live world holds, with where it stands.
pub(crate) fn a_player_ganger(app: &App) -> Option<(Entity, CellLevelNet)> {
    let world = app.world();
    let player = **world.get_resource::<PlayerFaction>()?;
    let mut gangers: Vec<(Entity, Position)> = world
        .iter_entities()
        .filter_map(|entity| {
            let faction = *entity.get::<Faction>()?;
            let position = *entity.get::<Position>()?;
            (faction == player).then_some((entity.id(), position))
        })
        .collect();
    gangers.sort_unstable_by_key(|(entity, _)| entity.to_bits());
    gangers
        .first()
        .map(|(entity, position)| (*entity, CellLevelNet::from_sim(**position)))
}

/// Every ganger the player commands, in the order their entity ids run.
pub(crate) fn player_gangers(app: &App) -> Vec<Entity> {
    let world = app.world();
    let Some(player) = world.get_resource::<PlayerFaction>().map(|player| **player) else {
        return Vec::new();
    };
    let mut gangers: Vec<Entity> = world
        .iter_entities()
        .filter_map(|entity| {
            let faction = *entity.get::<Faction>()?;
            let alive = entity
                .get::<LifeState>()
                .is_none_or(|life| *life.is_active());
            (faction == player && alive).then_some(entity.id())
        })
        .collect();
    gangers.sort_unstable_by_key(|entity| entity.to_bits());
    gangers
}

/// A player ganger the game has not already selected, so selecting it is a visible change.
pub(crate) fn an_unselected_player_ganger(app: &App) -> Option<Entity> {
    let already = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected);
    player_gangers(app)
        .into_iter()
        .find(|entity| Some(*entity) != already)
}

/// The first living enemy ganger the live world holds.
pub(crate) fn an_enemy_ganger(app: &App) -> Option<Entity> {
    an_enemy_ganger_at(app).map(|(entity, _)| entity)
}

/// The gang a living enemy fights for, which is never the gang the player commands.
pub(crate) fn an_enemy_faction(app: &App) -> Option<Faction> {
    let entity = an_enemy_ganger(app)?;
    app.world()
        .get_entity(entity)
        .ok()?
        .get::<Faction>()
        .copied()
}

/// The first living enemy ganger, with where it stands.
pub(crate) fn an_enemy_ganger_at(app: &App) -> Option<(Entity, CellLevelNet)> {
    let world = app.world();
    let player = **world.get_resource::<PlayerFaction>()?;
    let mut enemies: Vec<(Entity, Position)> = world
        .iter_entities()
        .filter_map(|entity| {
            let faction = *entity.get::<Faction>()?;
            let position = *entity.get::<Position>()?;
            let alive = entity
                .get::<LifeState>()
                .is_none_or(|life| *life.is_active());
            (faction != player && alive).then_some((entity.id(), position))
        })
        .collect();
    enemies.sort_unstable_by_key(|(entity, _)| entity.to_bits());
    enemies
        .first()
        .map(|(entity, position)| (*entity, CellLevelNet::from_sim(**position)))
}

/// The eight directions a step can take, as cell offsets.
const AROUND: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// Whether the map holds `step` as open ground with nothing standing on it or blocking it.
fn is_clear(grid: &OccupancyGrid, map: &HashSet<CellLevel>, step: &CellLevel) -> bool {
    map.contains(step)
        && grid.occupant(step).is_none()
        && !*grid.is_path_blocked(step)
        && matches!(grid.terrain(step), TerrainKind::Open)
}

/// A cell one clear step from `at`, when the map offers one.
pub(crate) fn one_step_from(app: &App, at: CellLevelNet) -> Option<CellLevelNet> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let map: HashSet<CellLevel> = grid.all_cells().collect();
    let (cell, level) = at.to_sim().split();
    AROUND.iter().find_map(|(east, north)| {
        let step = CellLevel::new(Cell::new(cell.x + east, cell.y + north), level);
        is_clear(grid, &map, &step).then_some(CellLevelNet::from_sim(step))
    })
}

/// A cell one clear step from `at` that no vertical link leads off, when the map offers one.
/// The game's left-click decision does nothing on a cell a ladder or a drop starts from.
pub(crate) fn one_step_from_without_a_link(app: &App, at: CellLevelNet) -> Option<CellLevelNet> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let links = app.world().get_resource::<VerticalLinkGraph>()?;
    let map: HashSet<CellLevel> = grid.all_cells().collect();
    let (cell, level) = at.to_sim().split();
    AROUND.iter().find_map(|(east, north)| {
        let step = CellLevel::new(Cell::new(cell.x + east, cell.y + north), level);
        (is_clear(grid, &map, &step) && links.links_from(&step).next().is_none())
            .then_some(CellLevelNet::from_sim(step))
    })
}

/// A clear cell one diagonal step from `at`, when the map offers one.
///
/// The two cells sharing that diagonal's corners come back with it, which is where a wall has
/// to stand to block the line between them.
pub(crate) fn a_clear_diagonal_from(app: &App, at: CellLevelNet) -> Option<DiagonalStep> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let map: HashSet<CellLevel> = grid.all_cells().collect();
    let (cell, level) = at.to_sim().split();
    [(1, 1), (1, -1), (-1, 1), (-1, -1)]
        .iter()
        .find_map(|(east, north)| {
            let step = CellLevel::new(Cell::new(cell.x + east, cell.y + north), level);
            let corners = [
                CellLevel::new(Cell::new(cell.x + east, cell.y), level),
                CellLevel::new(Cell::new(cell.x, cell.y + north), level),
            ];
            (is_clear(grid, &map, &step) && corners.iter().all(|corner| map.contains(corner)))
                .then_some(DiagonalStep { step, corners })
        })
}

/// A diagonal neighbour of a cell, with the two cells its line of sight is squeezed between.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DiagonalStep {
    pub(crate) step:    CellLevel,
    pub(crate) corners: [CellLevel; 2],
}

/// A cell two clear steps from `at` along one straight line, when the map offers one.
pub(crate) fn two_steps_from(app: &App, at: CellLevelNet) -> Option<CellLevelNet> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let map: HashSet<CellLevel> = grid.all_cells().collect();
    let (cell, level) = at.to_sim().split();
    AROUND.iter().find_map(|(east, north)| {
        let first = CellLevel::new(Cell::new(cell.x + east, cell.y + north), level);
        let second = CellLevel::new(Cell::new(cell.x + east * 2, cell.y + north * 2), level);
        (is_clear(grid, &map, &first) && is_clear(grid, &map, &second))
            .then_some(CellLevelNet::from_sim(second))
    })
}

/// Every living enemy standing 8-adjacent to `at`, in the order their entity ids run.
pub(crate) fn enemies_beside(app: &App, at: CellLevel) -> Vec<Entity> {
    let world = app.world();
    let Some(player) = world.get_resource::<PlayerFaction>().map(|player| **player) else {
        return Vec::new();
    };
    let mut beside: Vec<Entity> = world
        .iter_entities()
        .filter_map(|entity| {
            let faction = *entity.get::<Faction>()?;
            let position = *entity.get::<Position>()?;
            let alive = entity
                .get::<LifeState>()
                .is_none_or(|life| *life.is_active());
            (faction != player && alive && *is_8_adjacent(Position::new(at), position))
                .then_some(entity.id())
        })
        .collect();
    beside.sort_unstable_by_key(|entity| entity.to_bits());
    beside
}

/// Clear cells the map holds out of reach of `at`, so nothing stood there is 8-adjacent to it.
pub(crate) fn clear_cells_away_from(app: &App, at: CellLevel) -> Vec<CellLevel> {
    let Some(grid) = app.world().get_resource::<OccupancyGrid>() else {
        return Vec::new();
    };
    let map: HashSet<CellLevel> = grid.all_cells().collect();
    let mut away: Vec<CellLevel> = map
        .iter()
        .copied()
        .filter(|step| {
            is_clear(grid, &map, step) && !*is_8_adjacent(Position::new(at), Position::new(*step))
        })
        .collect();
    away.sort_unstable_by_key(|step| (step.x, step.y, step.z));
    away
}

/// A cell far outside any generated map, so nothing can be standing on it.
pub(crate) fn an_unreachable_cell() -> CellLevelNet {
    CellLevelNet::from_sim(CellLevel::new(Cell::new(-9_999, -9_999), Level::new(0)))
}

/// Render a cell as the compact RON body an argument field takes.
pub(crate) fn cell_argument(at: CellLevelNet) -> String {
    let Ok(text) = ron::ser::to_string(&at) else {
        unreachable!("a wire cell serializes to compact RON");
    };
    format!("(at:{text})")
}

/// The token a ganger is named by on the wire.
pub(crate) const fn token_of(entity: Entity) -> GangerToken {
    GangerToken::new(entity.to_bits())
}

/// Render a ganger token as the compact RON body `act.select` takes.
pub(crate) fn ganger_argument(entity: Entity) -> String {
    let Ok(text) = ron::ser::to_string(&token_of(entity)) else {
        unreachable!("a wire token serializes to compact RON");
    };
    format!("(ganger:{text})")
}

/// The cell a ganger is standing on right now.
pub(crate) fn cell_of(app: &App, entity: Entity) -> Option<CellLevelNet> {
    let position = *app.world().get_entity(entity).ok()?.get::<Position>()?;
    Some(CellLevelNet::from_sim(*position))
}

/// Which way a ganger is facing right now, as the wire spells it.
pub(crate) fn facing_of(app: &App, entity: Entity) -> Option<FacingNet> {
    app.world()
        .get_entity(entity)
        .ok()?
        .get::<Facing>()
        .map(|facing| FacingNet::from_sim(**facing))
}

/// Whether a ganger is aiming right now, read straight off the sim's own component.
pub(crate) fn aiming_of(app: &App, entity: Entity) -> Option<AimNet> {
    app.world()
        .get_entity(entity)
        .ok()?
        .get::<Aiming>()
        .map(|aiming| AimNet::new(**aiming))
}
