//! What the inspect panel shows for one cell.

use bevy::prelude::*;
use gdtf_battle_presenter::{CellVisibility, cell_squad_visible};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::PlayerFaction,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    occupancy::TerrainKind,
    prelude::{CellLevel, Faction, OccupancyGrid},
    visibility::{FactionRelation, SquadVisibility},
};

/// The playback-gated map the inspect panel reads, borrowed as one value.
#[derive(Clone, Copy)]
pub(crate) struct ShownBattle<'a> {
    grid:   Option<&'a OccupancyGrid>,
    cover:  Option<&'a CoverLedger>,
    fog:    Option<&'a SquadVisibility>,
    player: Option<&'a PlayerFaction>,
}

impl<'a> ShownBattle<'a> {
    /// Borrow the shown grid, cover ledger, fog and player faction together.
    pub(crate) const fn new(
        grid: Option<&'a OccupancyGrid>,
        cover: Option<&'a CoverLedger>,
        fog: Option<&'a SquadVisibility>,
        player: Option<&'a PlayerFaction>,
    ) -> Self {
        Self {
            grid,
            cover,
            fog,
            player,
        }
    }

    /// The shown occupancy grid.
    #[must_use]
    pub(crate) const fn grid(self) -> Option<&'a OccupancyGrid> {
        self.grid
    }

    /// The shown cover ledger.
    #[must_use]
    pub(crate) const fn cover(self) -> Option<&'a CoverLedger> {
        self.cover
    }

    /// The shown squad fog.
    #[must_use]
    pub(crate) const fn fog(self) -> Option<&'a SquadVisibility> {
        self.fog
    }

    /// The faction the player commands.
    #[must_use]
    pub(crate) const fn player(self) -> Option<&'a PlayerFaction> {
        self.player
    }

    /// How a faction relates to the player's own squad.
    #[must_use]
    pub(crate) fn relation(self, faction: Faction) -> FactionRelation {
        match self.player {
            Some(player) if faction == **player => FactionRelation::OwnSquad,
            Some(_) | None => FactionRelation::Other,
        }
    }

    /// Whether the squad can see a ganger of `faction` standing on `cell`.
    #[must_use]
    pub(crate) fn ganger_visible(self, cell: CellLevel, faction: Faction) -> CellVisibility {
        cell_squad_visible(self.fog, &cell, Some(self.relation(faction)))
    }

    /// Whether the squad can see `cell` itself.
    #[must_use]
    pub(crate) fn cell_visible(self, cell: CellLevel) -> CellVisibility {
        cell_squad_visible(self.fog, &cell, None)
    }
}

/// What the inspect panel is showing for a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectShown {
    /// A squad-visible ganger the panel draws a card for.
    Ganger(Entity),
    /// Destructible terrain the panel draws a cover block for.
    Cover(CoverEntry),
    /// Nothing; the panel hides.
    Nothing,
}

/// Decide what the inspect panel shows for `cell`.
///
/// `card_faction` answers with a ganger's faction when the panel can draw its card.
pub(crate) fn inspect_shown(
    cell: Option<CellLevel>,
    shown: ShownBattle<'_>,
    card_faction: impl Fn(Entity) -> Option<Faction>,
) -> InspectShown {
    let ganger = cell.and_then(|at| {
        let occupant = shown.grid()?.occupant(&at)?;
        let faction = card_faction(occupant)?;
        shown
            .ganger_visible(at, faction)
            .is_squad_visible()
            .then_some(occupant)
    });
    if let Some(ganger) = ganger {
        return InspectShown::Ganger(ganger);
    }
    match cell.and_then(|at| object_entry(at, shown.grid(), shown.cover())) {
        Some(entry) => InspectShown::Cover(entry),
        None => InspectShown::Nothing,
    }
}

/// The cover entry a wall or cover cell shows, seeded when the ledger has none.
#[must_use]
pub(crate) fn object_entry(
    cell: CellLevel,
    grid: Option<&OccupancyGrid>,
    ledger: Option<&CoverLedger>,
) -> Option<CoverEntry> {
    let grid = grid?;
    let terrain = grid.terrain(&cell);
    if !matches!(terrain, TerrainKind::Wall | TerrainKind::Cover) {
        return None;
    }
    ledger.and_then(|l| l.peek(&cell).copied()).or_else(|| {
        Some(CoverEntry::seeded(
            CoverHp::new(1),
            HeightBand::High,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
        ))
    })
}
