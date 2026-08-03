use bevy::prelude::Deref;
use serde::Deserialize;

/// blow), never this magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar;
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct PerStoreyDamage(i32);

impl PerStoreyDamage {
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
