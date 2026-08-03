use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::CellLevelNet,
    key::KeyPressNet,
    misc::FireModeIndex,
    pointer::PointerPosNet,
    token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
};

#[derive(
    Deref,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(transparent)]
pub struct ActSeqNet(u64);

impl ActSeqNet {
        #[must_use]
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum NetIntent {
                    Fire {
                        target: CellLevelNet,
                mode:   FireModeIndex,
    },
            Move {
                        dest: CellLevelNet,
    },
        SetStance {
                stance: StanceNet,
    },
            SetAiming {
                aim: AimNet,
    },
        SetFacing {
                facing: FacingNet,
    },
        Reload,
        EndTurn,
            Melee {
                target: MeleeTargetNet,
    },
        Shove {
                target: GangerToken,
    },
            Stabilize {
                target: GangerToken,
    },
        Execute {
                target: GangerToken,
    },
            ThrowGrenade {
                        target: CellLevelNet,
    },
        OpenDoor {
                target: DoorToken,
    },
            EnterEmplacement {
                target: EmplacementToken,
    },
        ExitEmplacement {
                target: EmplacementToken,
    },
            Select {
                target: GangerToken,
    },
        SelectNext,
            SelectPrev,
        SelectionClear,
        LevelUp,
        LevelDown,
                PressKey {
                key: KeyPressNet,
    },
                Hover {
                at: PointerPosNet,
    },
                    SetFocus {
                target: FocusTargetNet,
    },
}
