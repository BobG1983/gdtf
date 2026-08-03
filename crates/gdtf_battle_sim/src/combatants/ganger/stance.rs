//! Stance, aiming, and faction markers.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// Standing / crouching / prone.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum StanceKind {
    /// Upright.
    #[default]
    Standing,
    /// Crouched.
    Crouching,
    /// Prone.
    Prone,
}

/// Current posture.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Stance(StanceKind);

impl Stance {
    /// Wrap a stance kind.
    #[must_use]
    pub const fn new(posture: StanceKind) -> Self {
        Self(posture)
    }
}

/// Whether the ganger is aiming.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Aiming(bool);

impl Aiming {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(aiming: bool) -> Self {
        Self(aiming)
    }
}

/// Team / gang index (0 or 1 in the two-faction model).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Faction(u8);

impl Faction {
    /// Wrap a gang index.
    #[must_use]
    pub const fn new(gang: u8) -> Self {
        Self(gang)
    }
}
