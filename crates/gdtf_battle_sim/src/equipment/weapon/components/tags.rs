//! Boolean weapon tags.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Weapon can shove on hit.
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Shove(bool);

impl Shove {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(shove: bool) -> Self {
        Self(shove)
    }
}

/// Weapon is silent (no reaction from noise).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct Silenced(bool);

impl Silenced {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(silenced: bool) -> Self {
        Self(silenced)
    }
}

impl Default for Silenced {
    fn default() -> Self {
        Self(true)
    }
}
