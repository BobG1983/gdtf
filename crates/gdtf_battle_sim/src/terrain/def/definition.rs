//! Authored terrain piece definition.

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{LosBlocking, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid};

/// Display name for tooling and authoring.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct TerrainDisplayName(String);

impl TerrainDisplayName {
    /// Wrap a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Optional override for path blocking.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BlocksPathingOverride(bool);

impl BlocksPathingOverride {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(over: bool) -> Self {
        Self(over)
    }
}

/// One terrain piece in the catalog.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct TerrainDef {
    /// Content key.
    pub key:            TerrainUuid,
    /// Display name.
    pub display_name:   TerrainDisplayName,
    /// Simulation behaviour.
    pub sim_kind:       TerrainSimKind,
    /// Presenter behaviour.
    pub presenter_kind: TerrainPresenterKind,
    /// Optional tags.
    #[serde(default)]
    pub tags:           Vec<TerrainTag>,
    /// Optional on-death effect.
    #[serde(default)]
    pub on_death:       Option<crate::effects::on_death::OnDeathEffect>,
    /// Optional path-blocking override.
    #[serde(default)]
    pub blocks_pathing: Option<BlocksPathingOverride>,
    /// Optional LOS-blocking override.
    #[serde(default)]
    pub blocks_los:     Option<LosBlocking>,
}
