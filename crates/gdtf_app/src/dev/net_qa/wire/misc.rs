//! Misc scalar and command payloads on the wire.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Index into the shooter's fire-mode list.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AutoRunNet(bool);

impl AutoRunNet {
    /// Build from a bool.
    #[must_use]
    pub const fn new(running: bool) -> Self {
        Self(running)
    }
}

/// Procgen stepper command on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
