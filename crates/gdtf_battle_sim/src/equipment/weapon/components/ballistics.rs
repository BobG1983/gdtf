//! Spread, accuracy, kickback, stability.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Base cone spread in radians.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct BaseSpread(f32);

impl BaseSpread {
    /// Wrap a spread value.
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// Accuracy contribution to hit chance.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Accuracy(f32);

impl Accuracy {
    /// Wrap an accuracy value.
    #[must_use]
    pub const fn new(accuracy: f32) -> Self {
        Self(accuracy)
    }
}

/// Recoil / kick magnitude.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Kickback(f32);

impl Kickback {
    /// Wrap a kickback value.
    #[must_use]
    pub const fn new(kickback: f32) -> Self {
        Self(kickback)
    }
}

/// Whether the weapon is inherently stable.
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Stable(bool);

impl Stable {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(stable: bool) -> Self {
        Self(stable)
    }
}
