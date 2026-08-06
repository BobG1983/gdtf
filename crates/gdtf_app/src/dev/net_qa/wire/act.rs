//! Act sequence, act replies, and the network intent enum.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::CellLevelNet,
    key::KeyPressNet,
    misc::FireModeIndex,
    pointer::PointerPosNet,
    token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
};

/// Monotonic act sequence number on the wire.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ActSeqNet(u64);

impl ActSeqNet {
    /// Build from a raw sequence value.
    #[must_use]
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }
}

/// Whether the act finished inside the frame, rather than still walking itself out.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActCompleteNet(bool);

impl ActCompleteNet {
    /// Build from a bool.
    #[must_use]
    pub const fn new(complete: bool) -> Self {
        Self(complete)
    }
}

/// Why the QA layer turned an act away before the sim ever saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActRefusalNet {
    /// The token named no live ganger.
    UnknownToken,
    /// The act needs a selected shooter and nothing is selected.
    NoShooter,
}

/// What every classic act answers: the act-log window it opened, or a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActReply {
    /// The intent reached the sim; read `from_seq..to_seq` from the act log for what it did.
    Accepted {
        /// Act-log head when the call was claimed.
        from_seq: ActSeqNet,
        /// Act-log head once the sim had run that frame.
        to_seq:   ActSeqNet,
        /// False while the actor is still walking the act out.
        complete: ActCompleteNet,
    },
    /// The call never reached the sim.
    Refused {
        /// Why it was turned away.
        reason: ActRefusalNet,
    },
}

/// What the four selection commands answer: who is selected now, or a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelectReply {
    /// The selection as it stands after the command ran.
    Selected {
        /// Selected shooter, absent when nothing is selected.
        shooter: Option<GangerToken>,
    },
    /// The call never reached the selection.
    Refused {
        /// Why it was turned away.
        reason: ActRefusalNet,
    },
}

/// Player or automation intent sent over the QA channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetIntent {
    /// Fire at a cell with a fire-mode index.
    Fire {
        /// Target cell-level.
        target: CellLevelNet,
        /// Selected fire mode index.
        mode:   FireModeIndex,
    },
    /// Move to a destination cell.
    Move {
        /// Destination cell-level.
        dest: CellLevelNet,
    },
    /// Change stance.
    SetStance {
        /// New stance.
        stance: StanceNet,
    },
    /// Toggle or set aiming.
    SetAiming {
        /// Aiming on/off.
        aim: AimNet,
    },
    /// Change facing.
    SetFacing {
        /// New facing.
        facing: FacingNet,
    },
    /// Reload the active weapon.
    Reload,
    /// End the current turn.
    EndTurn,
    /// Melee attack.
    Melee {
        /// Melee target.
        target: MeleeTargetNet,
    },
    /// Shove a ganger.
    Shove {
        /// Target ganger.
        target: GangerToken,
    },
    /// Stabilize a ganger.
    Stabilize {
        /// Target ganger.
        target: GangerToken,
    },
    /// Execute a downed ganger.
    Execute {
        /// Target ganger.
        target: GangerToken,
    },
    /// Throw a grenade.
    ThrowGrenade {
        /// Impact cell-level.
        target: CellLevelNet,
    },
    /// Open a door.
    OpenDoor {
        /// Door token.
        target: DoorToken,
    },
    /// Enter an emplacement.
    EnterEmplacement {
        /// Emplacement token.
        target: EmplacementToken,
    },
    /// Exit an emplacement.
    ExitEmplacement {
        /// Emplacement token.
        target: EmplacementToken,
    },
    /// Select a ganger.
    Select {
        /// Target ganger.
        target: GangerToken,
    },
    /// Select the next player ganger.
    SelectNext,
    /// Select the previous player ganger.
    SelectPrev,
    /// Clear selection.
    SelectionClear,
    /// Step view one level up.
    LevelUp,
    /// Step view one level down.
    LevelDown,
    /// Synthetic key press.
    PressKey {
        /// Key or keybind action.
        key: KeyPressNet,
    },
    /// Move the pointer.
    Hover {
        /// Pointer position.
        at: PointerPosNet,
    },
    /// Set UI focus target.
    SetFocus {
        /// Focus target token.
        target: FocusTargetNet,
    },
}
