//! Act, legality and refusal payloads for a TU cost preview.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::CellLevelNet,
    misc::ModeKindNet,
    token::{DoorToken, EmplacementToken, GangerToken},
};

/// One act a cost can be asked about, carrying what it would act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostActNet {
    /// Walk to a cell.
    Move {
        /// Cell the walk ends on.
        dest: CellLevelNet,
    },
    /// Fire on a cell with one of the wielded gun's modes.
    Fire {
        /// Cell fired at.
        target: CellLevelNet,
        /// Fire mode used.
        mode:   ModeKindNet,
    },
    /// Refill the wielded gun's magazine.
    Reload,
    /// Take a stance.
    SetStance {
        /// Stance to take.
        stance: StanceNet,
    },
    /// Turn to face a direction.
    SetFacing {
        /// Direction to face.
        facing: FacingNet,
    },
    /// Take or drop aim.
    SetAiming {
        /// Whether to aim.
        aim: AimNet,
    },
    /// Shove an adjacent ganger.
    Shove {
        /// Ganger shoved.
        target: GangerToken,
    },
    /// Open an adjacent door.
    OpenDoor {
        /// Door opened.
        target: DoorToken,
    },
    /// Take an adjacent empty emplacement.
    EnterEmplacement {
        /// Emplacement entered.
        target: EmplacementToken,
    },
    /// Leave the emplacement being manned.
    ExitEmplacement {
        /// Emplacement left.
        target: EmplacementToken,
    },
    /// Throw the wielded arc weapon at a cell.
    ThrowGrenade {
        /// Cell thrown at.
        target: CellLevelNet,
    },
    /// Strike an adjacent ganger or structure.
    Melee {
        /// What the strike lands on.
        target: MeleeTargetNet,
    },
}

/// Whether the act would be allowed right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CostLegalNet(bool);

impl CostLegalNet {
    /// Build from a legality flag.
    #[must_use]
    pub const fn new(legal: bool) -> Self {
        Self(legal)
    }

    /// Legal exactly when nothing refused the act.
    #[must_use]
    pub const fn from_refusal(refusal: Option<CostRefusalNet>) -> Self {
        Self(refusal.is_none())
    }
}

/// Why an act would not be allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostRefusalNet {
    /// A token named nothing the world still holds.
    NoSuchGanger,
    /// The actor is not one of the player's gangers.
    NotYourGanger,
    /// No route reaches the cell.
    NoPathToCell,
    /// Suppression holds the mover and the route does not break away from it.
    Suppressed,
    /// Cover blocks the attacker's line of sight to the target.
    NoLineOfSight,
    /// The actor cannot pay the cost.
    CannotAfford,
    /// The act's own sim legality check said no.
    ActNotAllowed,
}
