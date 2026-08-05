//! Contextual act offers on the wire.

use serde::{Deserialize, Serialize};

use super::{
    cell::CellLevelNet,
    token::{DoorToken, EmplacementToken, GangerToken},
};

/// Which contextual act a panel button offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContextualActNet {
    /// Finish off an adjacent downed ganger.
    Execute,
    /// Stop an adjacent squadmate bleeding out.
    Stabilize,
    /// Swing at an adjacent target.
    Melee,
    /// Shove an adjacent ganger.
    Shove,
    /// Open an adjacent door.
    OpenDoor,
    /// Mount an adjacent emplacement.
    EnterEmplacement,
    /// Dismount the occupied emplacement.
    ExitEmplacement,
    /// Throw at the inspected cell.
    ThrowGrenade,
}

/// What an offered act would act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OfferTargetNet {
    /// A ganger.
    Ganger(GangerToken),
    /// A door.
    Door(DoorToken),
    /// An emplacement.
    Emplacement(EmplacementToken),
    /// A cell.
    Cell(CellLevelNet),
}

/// One contextual button the panel is offering right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextualOfferNet {
    /// The act on offer.
    pub act:    ContextualActNet,
    /// What it would act on.
    pub target: OfferTargetNet,
}

impl ContextualOfferNet {
    /// Build from an act and its target.
    #[must_use]
    pub const fn new(act: ContextualActNet, target: OfferTargetNet) -> Self {
        Self { act, target }
    }
}
