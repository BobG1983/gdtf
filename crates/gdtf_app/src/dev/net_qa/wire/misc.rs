use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FireModeIndex(u32);

impl FireModeIndex {
        #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SituationRef(String);

impl SituationRef {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SeedNet(u64);

impl SeedNet {
        #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FrameDelay(u32);

impl FrameDelay {
        #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RequestId(u64);

impl RequestId {
        #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AutoRunNet(bool);

impl AutoRunNet {
        #[must_use]
    pub const fn new(running: bool) -> Self {
        Self(running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum StepperCommandNet {
        Next,
            Auto {
                running: AutoRunNet,
    },
        Skip,
}
