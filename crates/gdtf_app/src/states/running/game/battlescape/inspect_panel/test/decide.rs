use bevy::{ecs::entity::Entity, platform::collections::HashSet, prelude::World};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    cover::CoverLedger,
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Faction, Level, OccupancyGrid},
    visibility::SquadVisibility,
};

use crate::states::running::game::battlescape::inspect_panel::decide::{
    InspectShown, ShownBattle, inspect_shown,
};

const PLAYER: Faction = Faction::new(0);

const ENEMY: Faction = Faction::new(1);

fn cell(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn lit(cells: &[CellLevel]) -> SquadVisibility {
    let seen: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(seen.clone(), seen)
}

fn an_entity() -> Entity {
    World::new().spawn_empty().id()
}

#[test]
fn a_ganger_the_squad_can_see_gets_its_card() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(at, Some(occupant));
    let fog = lit(&[at]);
    let player = PlayerFaction::new(PLAYER);
    let ledger = CoverLedger::new();
    let shown = ShownBattle::new(Some(&grid), Some(&ledger), Some(&fog), Some(&player));

    assert_eq!(
        inspect_shown(Some(at), shown, |_| Some(ENEMY)),
        InspectShown::Ganger(occupant),
        "the cell is in the squad's field of view, so the panel draws the occupant's card",
    );
}

#[test]
fn a_ganger_the_fog_hides_gets_no_card() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(at, Some(occupant));
    let fog = lit(&[cell(9, 9)]);
    let player = PlayerFaction::new(PLAYER);
    let ledger = CoverLedger::new();
    let shown = ShownBattle::new(Some(&grid), Some(&ledger), Some(&fog), Some(&player));

    assert_eq!(
        inspect_shown(Some(at), shown, |_| Some(ENEMY)),
        InspectShown::Nothing,
        "the fog hides this cell, so the panel draws nothing for the ganger standing on it",
    );
}

#[test]
fn an_own_squad_ganger_shows_through_the_fog() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(at, Some(occupant));
    let fog = lit(&[]);
    let player = PlayerFaction::new(PLAYER);
    let ledger = CoverLedger::new();
    let shown = ShownBattle::new(Some(&grid), Some(&ledger), Some(&fog), Some(&player));

    assert_eq!(
        inspect_shown(Some(at), shown, |_| Some(PLAYER)),
        InspectShown::Ganger(occupant),
        "own-squad members are always squad-visible, which is the sim's own rule",
    );
}

#[test]
fn a_hidden_ganger_still_lets_the_cover_block_through() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(at, Some(occupant));
    grid.set_terrain(at, TerrainKind::Cover);
    let fog = lit(&[]);
    let player = PlayerFaction::new(PLAYER);
    let ledger = CoverLedger::new();
    let shown = ShownBattle::new(Some(&grid), Some(&ledger), Some(&fog), Some(&player));

    assert!(
        matches!(
            inspect_shown(Some(at), shown, |_| Some(ENEMY)),
            InspectShown::Cover(_)
        ),
        "with the ganger hidden the panel falls through to the terrain the cell holds",
    );
}

#[test]
fn an_empty_open_cell_shows_nothing() {
    let at = cell(3, 3);
    let grid = OccupancyGrid::new();
    let fog = lit(&[at]);
    let player = PlayerFaction::new(PLAYER);
    let ledger = CoverLedger::new();
    let shown = ShownBattle::new(Some(&grid), Some(&ledger), Some(&fog), Some(&player));

    assert_eq!(
        inspect_shown(Some(at), shown, |_| Some(ENEMY)),
        InspectShown::Nothing,
        "no ganger and no terrain leaves the panel hidden",
    );
}
