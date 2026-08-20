//! What the inspect panel shows for one cell.

use bevy::prelude::*;
use gdtf_battle_presenter::{CellVisibility, cell_squad_visible};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    cover::{CoverEntry, CoverLedger},
    entity::TerrainPieceKind,
    occupancy::TerrainKind,
    prelude::{CellLevel, Faction, OccupancyGrid},
    visibility::{FactionRelation, SquadVisibility},
};

use super::shadow::{ShownEmplacement, ShownEmplacements};

/// The playback-gated map the inspect panel reads, borrowed as one value.
#[derive(Clone, Copy)]
pub(crate) struct ShownBattle<'a> {
    grid:         Option<&'a OccupancyGrid>,
    cover:        Option<&'a CoverLedger>,
    emplacements: Option<&'a ShownEmplacements>,
    fog:          Option<&'a SquadVisibility>,
    player:       Option<&'a PlayerFaction>,
}

impl<'a> ShownBattle<'a> {
    /// Borrow the shown grid, cover ledger, emplacements, fog and player faction together.
    pub(crate) const fn new(
        grid: Option<&'a OccupancyGrid>,
        cover: Option<&'a CoverLedger>,
        emplacements: Option<&'a ShownEmplacements>,
        fog: Option<&'a SquadVisibility>,
        player: Option<&'a PlayerFaction>,
    ) -> Self {
        Self {
            grid,
            cover,
            emplacements,
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

    /// The shown emplacements, by the cell each one stands on.
    #[must_use]
    pub(crate) const fn emplacements(self) -> Option<&'a ShownEmplacements> {
        self.emplacements
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

/// The terrain on an inspected cell: its piece kind, cover stats and emplacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectTerrain {
    kind:        TerrainPieceKind,
    cover:       Option<CoverEntry>,
    emplacement: Option<ShownEmplacement>,
}

impl InspectTerrain {
    /// Report a piece kind, with whatever cover stats and emplacement the cell holds.
    pub(crate) const fn new(
        kind: TerrainPieceKind,
        cover: Option<CoverEntry>,
        emplacement: Option<ShownEmplacement>,
    ) -> Self {
        Self {
            kind,
            cover,
            emplacement,
        }
    }

    /// Which kind of piece stands on the cell.
    #[must_use]
    pub(crate) const fn kind(&self) -> TerrainPieceKind {
        self.kind
    }

    /// The cover stats the ledger holds for the cell, if it holds any.
    #[must_use]
    pub(crate) const fn cover(&self) -> Option<CoverEntry> {
        self.cover
    }

    /// The emplacement standing on the cell, if there is one.
    #[must_use]
    pub(crate) const fn emplacement(&self) -> Option<&ShownEmplacement> {
        self.emplacement.as_ref()
    }
}

/// What the inspect panel reports for a cell: the ganger on it and the terrain under it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct InspectShown {
    ganger:  Option<Entity>,
    terrain: Option<InspectTerrain>,
}

impl InspectShown {
    /// Report both halves; each is absent when the cell has nothing to say about it.
    pub(crate) const fn new(ganger: Option<Entity>, terrain: Option<InspectTerrain>) -> Self {
        Self { ganger, terrain }
    }

    /// The ganger whose card the panel draws.
    #[must_use]
    pub(crate) const fn ganger(&self) -> Option<Entity> {
        self.ganger
    }

    /// The terrain the panel draws an object block for.
    #[must_use]
    pub(crate) const fn terrain(&self) -> Option<&InspectTerrain> {
        self.terrain.as_ref()
    }
}

/// Decide what the inspect panel reports for `cell`.
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
    let terrain = cell
        .filter(|at| shown.cell_visible(*at).is_squad_visible())
        .and_then(|at| object_entry(at, shown));
    InspectShown::new(ganger, terrain)
}

/// What the terrain on a cell reports, or nothing when the cell is open floor.
#[must_use]
pub(crate) fn object_entry(cell: CellLevel, shown: ShownBattle<'_>) -> Option<InspectTerrain> {
    let kind = match shown.grid()?.terrain(&cell) {
        TerrainKind::Wall => TerrainPieceKind::Wall,
        TerrainKind::Cover => TerrainPieceKind::Cover,
        TerrainKind::Emplacement => TerrainPieceKind::Emplacement,
        TerrainKind::Open => return None,
    };
    Some(InspectTerrain::new(
        kind,
        shown.cover().and_then(|ledger| ledger.peek(&cell).copied()),
        shown
            .emplacements()
            .and_then(|seats| seats.peek(&cell).cloned()),
    ))
}
