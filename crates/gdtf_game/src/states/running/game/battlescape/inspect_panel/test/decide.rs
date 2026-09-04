use bevy::{ecs::entity::Entity, platform::collections::HashSet, prelude::World};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::PlayerFaction,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    emplacement::{EmplacementState, MountedWeaponKey},
    entity::TerrainPieceKind,
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Faction, Level, OccupancyGrid},
    visibility::SquadVisibility,
    weapon::WeaponName,
};

use crate::states::running::game::battlescape::inspect_panel::{
    decide::{InspectTerrain, ShownBattle, inspect_shown},
    shadow::{ShownEmplacement, ShownEmplacements},
};

const PLAYER: Faction = Faction::new(0);

const ENEMY: Faction = Faction::new(1);

const GUN: &str = "Heavy Stubber";

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

fn an_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(12),
        HeightBand::Mid,
        ArmorProtection::new(3),
        ArmorHardness::new(2),
        TerrainPieceKind::Cover,
    )
}

/// One cell's worth of shown battle, assembled piece by piece.
struct Shown {
    grid:   OccupancyGrid,
    ledger: CoverLedger,
    seats:  ShownEmplacements,
    fog:    SquadVisibility,
    player: PlayerFaction,
}

impl Shown {
    fn new() -> Self {
        Self {
            grid:   OccupancyGrid::new(),
            ledger: CoverLedger::new(),
            seats:  ShownEmplacements::default(),
            fog:    lit(&[]),
            player: PlayerFaction::new(PLAYER),
        }
    }

    fn view(&self) -> ShownBattle<'_> {
        ShownBattle::new(
            Some(&self.grid),
            Some(&self.ledger),
            Some(&self.seats),
            Some(&self.fog),
            Some(&self.player),
        )
    }
}

fn cover_at(at: CellLevel, entry: CoverEntry) -> Shown {
    let mut shown = Shown::new();
    shown.grid.set_terrain(at, TerrainKind::Cover);
    shown.ledger.insert(at, entry);
    shown
}

#[test]
fn a_ganger_the_squad_can_see_gets_its_card() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let mut shown = Shown::new();
    shown.grid.set_occupant(at, Some(occupant));
    shown.fog = lit(&[at]);

    assert_eq!(
        inspect_shown(Some(at), shown.view(), |_| Some(ENEMY)).ganger(),
        Some(occupant),
        "the cell is in the squad's field of view, so the panel draws the occupant's card",
    );
}

#[test]
fn a_ganger_the_fog_hides_gets_no_card() {
    let at = cell(3, 3);
    let mut shown = Shown::new();
    shown.grid.set_occupant(at, Some(an_entity()));
    shown.fog = lit(&[cell(9, 9)]);

    assert_eq!(
        inspect_shown(Some(at), shown.view(), |_| Some(ENEMY)).ganger(),
        None,
        "the fog hides this cell, so the panel draws nothing for the ganger standing on it",
    );
}

#[test]
fn an_own_squad_ganger_shows_through_the_fog() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let mut shown = Shown::new();
    shown.grid.set_occupant(at, Some(occupant));

    assert_eq!(
        inspect_shown(Some(at), shown.view(), |_| Some(PLAYER)).ganger(),
        Some(occupant),
        "own-squad members are always squad-visible, which is the sim's own rule",
    );
}

#[test]
fn an_empty_open_cell_shows_nothing() {
    let at = cell(3, 3);
    let mut shown = Shown::new();
    shown.fog = lit(&[at]);

    let report = inspect_shown(Some(at), shown.view(), |_| Some(ENEMY));
    assert_eq!(
        (report.ganger(), report.terrain()),
        (None, None),
        "no ganger and no terrain leaves the panel hidden",
    );
}

#[test]
fn a_ganger_and_the_cover_it_stands_in_are_both_reported() {
    let occupant = an_entity();
    let at = cell(3, 3);
    let entry = an_entry();
    let mut shown = cover_at(at, entry);
    shown.grid.set_occupant(at, Some(occupant));
    shown.fog = lit(&[at]);

    let report = inspect_shown(Some(at), shown.view(), |_| Some(ENEMY));
    assert_eq!(
        report.ganger(),
        Some(occupant),
        "the squad can see the cell, so the ganger half names its occupant: {report:?}",
    );
    assert_eq!(
        report.terrain().and_then(InspectTerrain::cover),
        Some(entry),
        "one cell reports everything on it, so the terrain half comes back too: {report:?}",
    );
}

#[test]
fn a_fog_hidden_occupant_changes_nothing_in_the_report() {
    let at = cell(3, 3);
    let entry = an_entry();
    let mut occupied = cover_at(at, entry);
    occupied.grid.set_occupant(at, Some(an_entity()));
    let empty = cover_at(at, entry);

    assert_eq!(
        inspect_shown(Some(at), occupied.view(), |_| Some(ENEMY)),
        inspect_shown(Some(at), empty.view(), |_| Some(ENEMY)),
        "a hidden enemy must read exactly as an empty cell does, or the report tells a \
         caller somebody is standing there",
    );

    let mut seen = cover_at(at, entry);
    seen.fog = lit(&[at]);
    assert_eq!(
        inspect_shown(Some(at), seen.view(), |_| Some(ENEMY))
            .terrain()
            .and_then(InspectTerrain::cover),
        Some(entry),
        "the same cell lit and empty does report the cover, so the two hidden reports \
         match on the rule and not on having nothing to say",
    );
}

#[test]
fn an_emplacement_cell_reports_its_state_and_mounted_weapon() {
    let at = cell(4, 2);
    let mut shown = Shown::new();
    shown.grid.set_terrain(at, TerrainKind::Emplacement);
    shown.seats.promote(std::iter::once((
        at,
        ShownEmplacement::new(
            EmplacementState::Vacant,
            MountedWeaponKey::new(WeaponName::new(GUN.to_owned())),
        ),
    )));
    shown.fog = lit(&[at]);

    let report = inspect_shown(Some(at), shown.view(), |_| Some(ENEMY));
    let Some(seat) = report.terrain().and_then(InspectTerrain::emplacement) else {
        unreachable!("an emplacement cell reports a terrain half naming its seat: {report:?}")
    };
    assert_eq!(
        seat.state(),
        EmplacementState::Vacant,
        "the report names the seat's own state: {report:?}",
    );
    assert_eq!(
        **seat.weapon(),
        WeaponName::new(GUN.to_owned()),
        "the report names the weapon the seat mounts: {report:?}",
    );
}

#[test]
fn a_wall_the_ledger_has_no_entry_for_reports_its_kind_and_no_stats() {
    let at = cell(5, 5);
    let mut shown = Shown::new();
    shown.grid.set_terrain(at, TerrainKind::Wall);
    shown.fog = lit(&[at]);

    let report = inspect_shown(Some(at), shown.view(), |_| Some(ENEMY));
    assert_eq!(
        report.terrain().map(InspectTerrain::kind),
        Some(TerrainPieceKind::Wall),
        "the cell reports the kind of piece standing on it: {report:?}",
    );
    assert_eq!(
        report.terrain().and_then(InspectTerrain::cover),
        None,
        "no ledger entry means no HP, height band, protection or hardness — an unseeded \
         cell is told apart by the reply, not by the value of its numbers: {report:?}",
    );
}

#[test]
fn a_cell_the_squad_cannot_see_reports_no_terrain() {
    let at = cell(3, 3);
    let entry = an_entry();
    let shown = cover_at(at, entry);

    let report = inspect_shown(Some(at), shown.view(), |_| Some(ENEMY));
    let leaked = report
        .terrain()
        .and_then(InspectTerrain::cover)
        .map(|entry| *entry.armor_protection);
    assert_eq!(
        report.terrain(),
        None,
        "the fog does not light this cell, so it reports nothing about the cover standing \
         on it; protection {leaked:?} reached the caller instead",
    );
}
