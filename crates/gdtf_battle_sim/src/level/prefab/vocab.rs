use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum SpawnRole {
        Player,
        Enemy,
        Fill,
}

/// [`Deref`]; `#[serde(transparent)]` is NOT needed (a prefab file does not author its
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrefabName(String);

impl PrefabName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
