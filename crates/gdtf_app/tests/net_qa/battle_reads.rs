//! Shared world reads the battle-read socket cases plan their requests from.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::cell::CellLevelNet;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Faction, LifeState, Position},
    prelude::{Cell, CellLevel, Level},
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

/// The cell one step east of `at`, which is 8-adjacent to it.
pub(crate) fn beside(at: CellLevelNet) -> CellLevel {
    let at = at.to_sim();
    let (cell, level) = at.split();
    CellLevel::new(Cell::new(cell.x + 1, cell.y), level)
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
