//! Prefab names and spawn roles.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// How a prefab is used when assembling a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum SpawnRole {
    /// Player-side starting area.
    Player,
    /// Enemy-side starting area.
    Enemy,
    /// Fill / neutral terrain.
    Fill,
}

/// Stable name for a prefab.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrefabName(String);

impl PrefabName {
    /// Wrap a name string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
