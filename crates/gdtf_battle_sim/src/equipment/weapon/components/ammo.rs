//! Ammo class for magazine compatibility.

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

/// Magazine ammo class.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum AmmoType {
    /// Solid slug (default).
    #[default]
    Slug,
    /// Energy cell.
    Cell,
    /// Flask.
    Flask,
    /// Canister.
    Canister,
    /// Thrown grenade.
    Grenade,
}

impl AmmoType {
    /// All variants.
    pub const ALL: [Self; 5] = [
        Self::Slug,
        Self::Cell,
        Self::Flask,
        Self::Canister,
        Self::Grenade,
    ];
}
