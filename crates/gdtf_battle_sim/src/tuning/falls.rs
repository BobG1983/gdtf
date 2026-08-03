//! Fall damage per storey.

use bevy::prelude::Deref;
use serde::Deserialize;

/// HP damage applied per storey fallen.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct PerStoreyDamage(i32);

impl PerStoreyDamage {
    /// Wrap a damage value.
    #[must_use]
    pub const fn new(damage: i32) -> Self {
        Self(damage)
    }
}

impl Default for PerStoreyDamage {
    fn default() -> Self {
        Self(6)
    }
}
