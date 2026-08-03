//! `#[serde(transparent)]` `u64` says `{"type":"integer","format":"uint64"}` and nothing
use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct GangerToken(u64);

impl GangerToken {
        #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct DoorToken(u64);

impl DoorToken {
        #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EmplacementToken(u64);

impl EmplacementToken {
        #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FocusTargetNet(u64);

impl FocusTargetNet {
        #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}
