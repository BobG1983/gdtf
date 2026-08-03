//! Melee marker and reach.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Strike range in cells (default 1).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reach(u16);

impl Reach {
    /// Default reach of one cell.
    pub const DEFAULT: Self = Self(1);

    /// Wrap a reach value.
    #[must_use]
    pub const fn new(reach: u16) -> Self {
        Self(reach)
    }
}

impl Default for Reach {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Marks a melee weapon entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeleeWeapon;
