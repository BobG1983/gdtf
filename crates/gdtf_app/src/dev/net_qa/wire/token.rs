//! Opaque entity tokens on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Opaque ganger identity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangerToken(u64);

impl GangerToken {
    /// Build from raw bits.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// Opaque door identity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DoorToken(u64);

impl DoorToken {
    /// Build from raw bits.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// Opaque emplacement identity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmplacementToken(u64);

impl EmplacementToken {
    /// Build from raw bits.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// Opaque UI focus target identity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FocusTargetNet(u64);

impl FocusTargetNet {
    /// Build from raw bits.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}
