//! Contextual act offers on the wire.

use bevy::prelude::Deref;
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

/// Whether the panel is showing an offered button as pressable rather than greyed out.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OfferPressableNet(bool);

impl OfferPressableNet {
    /// Build from a pressable flag.
    #[must_use]
    pub const fn new(pressable: bool) -> Self {
        Self(pressable)
    }
}

/// One contextual button the panel is offering right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextualOfferNet {
    /// The act on offer.
    pub act:       ContextualActNet,
    /// What it would act on.
    pub target:    OfferTargetNet,
    /// False when the panel greys the button out: the pool cannot pay the act's cost, or,
    /// for melee, the actor wields no melee weapon.
    pub pressable: OfferPressableNet,
}

impl ContextualOfferNet {
    /// Build from an act, its target, and whether its button is pressable.
    #[must_use]
    pub const fn new(
        act: ContextualActNet,
        target: OfferTargetNet,
        pressable: OfferPressableNet,
    ) -> Self {
        Self {
            act,
            target,
            pressable,
        }
    }
}
