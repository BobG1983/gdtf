//! Misc scalar and command payloads on the wire.

use bevy::prelude::Deref;
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::{procgen::ProcgenStage, weapon::ModeKind};
use serde::{Deserialize, Serialize};

/// Which fire mode the shooter has selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModeKindNet {
    /// One shot.
    Single,
    /// Short burst.
    Burst,
    /// Full auto.
    Full,
}

impl ModeKindNet {
    /// Mirror the sim's fire-mode kind.
    #[must_use]
    pub const fn from_sim(kind: ModeKind) -> Self {
        match kind {
            ModeKind::Single => Self::Single,
            ModeKind::Burst => Self::Burst,
            ModeKind::Full => Self::Full,
        }
    }

    /// Convert back into the sim's fire-mode kind.
    #[must_use]
    pub const fn to_sim(self) -> ModeKind {
        match self {
            Self::Single => ModeKind::Single,
            Self::Burst => ModeKind::Burst,
            Self::Full => ModeKind::Full,
        }
    }
}

/// Which storeys the battle view is drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewModeNet {
    /// Only storeys at or below the active level.
    DownToActive,
    /// Every storey, with context treatment.
    FullView,
}

impl ViewModeNet {
    /// Mirror the presenter's view mode.
    #[must_use]
    pub const fn from_view(mode: ViewMode) -> Self {
        match mode {
            ViewMode::DownToActive => Self::DownToActive,
            ViewMode::FullView => Self::FullView,
        }
    }
}

/// Index into the shooter's fire-mode list.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FireModeIndex(u32);

impl FireModeIndex {
    /// Build from a raw index.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// Situation asset name reference.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SituationRef(String);

impl SituationRef {
    /// Build from a situation name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Battle seed on the wire.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeedNet(u64);

impl SeedNet {
    /// Build from a seed value.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }
}

/// Frame delay before an action.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameDelay(u32);

impl FrameDelay {
    /// Build from a frame count.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

/// Client request id.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(u64);

impl RequestId {
    /// Build from a raw id.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Whether the procgen stepper is auto-running.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoRunNet(bool);

impl AutoRunNet {
    /// Build from a bool.
    #[must_use]
    pub const fn new(running: bool) -> Self {
        Self(running)
    }
}

/// Which stage staged procgen is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcgenStageNet {
    /// Placing player and enemy spawns.
    Assemble,
    /// Filling free space with content prefabs.
    Fill,
    /// Emitting the situation.
    Emit,
    /// Finished, whether it succeeded or failed.
    Done,
}

impl ProcgenStageNet {
    /// Mirror the sim's own stage.
    #[must_use]
    pub const fn from_stage(stage: ProcgenStage) -> Self {
        match stage {
            ProcgenStage::Assemble => Self::Assemble,
            ProcgenStage::Fill => Self::Fill,
            ProcgenStage::Emit => Self::Emit,
            ProcgenStage::Done => Self::Done,
        }
    }
}

/// Procgen stepper command on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepperCommandNet {
    /// Advance one step.
    Next,
    /// Set auto-run.
    Auto {
        /// Whether auto-run is on.
        running: AutoRunNet,
    },
    /// Skip remaining steps.
    Skip,
}
