//! What a battle fixture reports back, so a case can check the reply against it.

use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    misc::ModeKindNet,
    roster::{FactionNet, GangerNameNet},
    token::GangerToken,
    vitals::{HpMaxNet, TuMaxNet},
};

/// Act-log lines the log fixture appends before the read.
pub(crate) const LOG_LINES_WRITTEN: u32 = 5;

/// Act-log lines the flooded fixture appends, more than the read's default window keeps.
pub(crate) const FLOODED_LOG_LINES: u32 = 256;

/// Whether a fixture stands its thing inside the squad's field of view or outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// Inside the lit area.
    Lit,
    /// Hidden by the fog.
    Hidden,
}

impl Standing {
    /// The other side of the fog line.
    pub(crate) const fn opposite(self) -> Self {
        match self {
            Self::Lit => Self::Hidden,
            Self::Hidden => Self::Lit,
        }
    }
}

/// Whether a fixture fills the shooter's magazine or empties it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MagazineLoad {
    /// Filled to capacity.
    Loaded,
    /// Emptied of every round.
    Empty,
}

/// An enemy the sim and the screen disagree about, with both cells.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SplitEnemy {
    pub(crate) entity: Entity,
    pub(crate) drawn:  CellLevelNet,
    pub(crate) live:   CellLevelNet,
}

/// An enemy standing where only the screen's frozen fog still reaches.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FrozenFog {
    pub(crate) entity: Entity,
    pub(crate) at:     CellLevelNet,
}

/// A cell the sim has given an occupant that the screen has not shown arriving.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LateOccupant {
    pub(crate) at: CellLevelNet,
}

/// The enemy a fixture named, and the cell a read must report it on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpectedEnemy {
    pub(crate) entity: Entity,
    pub(crate) at:     CellLevelNet,
}

/// A player ganger the game left unselected, and the enemy a fixture stood next to it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IdlePair {
    pub(crate) shooter: Entity,
    pub(crate) enemy:   Entity,
}

/// The openable a fixture spawned, and where it stands.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SpawnedDoor {
    pub(crate) entity: Entity,
    pub(crate) at:     CellLevelNet,
}

/// The fire mode a fixture selected for the shooter.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpectedFireMode {
    pub(crate) kind: ModeKindNet,
}

/// A player ganger's identity as the live world holds it, for the card to be checked against.
#[derive(Debug, Clone)]
pub(crate) struct LiveCard {
    pub(crate) token:   GangerToken,
    pub(crate) name:    Option<GangerNameNet>,
    pub(crate) faction: FactionNet,
    pub(crate) tu_max:  TuMaxNet,
    pub(crate) hp_max:  Option<HpMaxNet>,
}

/// A wall or cover cell the squad can see, which both cell reads must describe alike.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LitCover {
    pub(crate) at: CellLevelNet,
}

/// A cover cell the screen's fog still remembers but no longer lights.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RememberedCover {
    pub(crate) at: CellLevelNet,
}

/// The enemy a fixture stood in the dark and fired across the squad's lit area.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HiddenShooter {
    pub(crate) entity: Entity,
}

/// The ganger every act-log line the fixture wrote names.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoggedActor {
    pub(crate) actor: Entity,
}

/// The selected shooter a fixture posed, and the cell directly behind its facing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PosedShooter {
    pub(crate) behind: CellLevelNet,
}

/// The acting gang a fixture set, beside the gang the player commands.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpectedTurn {
    pub(crate) active: FactionNet,
    pub(crate) player: FactionNet,
}
