//! Act sequence and network intent enum.

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
