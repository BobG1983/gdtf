//! Prefab save error type and path helpers.

use gdtf_assets::RonSaveError;
use gdtf_battle_sim::{
    level::{GridSize, SpawnRole},
    metric::CellLevel,
};

pub(super) const SAVED_SPAWN_ROLE: SpawnRole = SpawnRole::Fill;

/// Errors from prefab validation or save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavePrefabError {
    /// Prefab name is empty after sanitization.
    EmptyName,
    /// A painted cell fails placement rules.
    IllegalCell(CellLevel),
    /// Underlying RON write failed.
    Save(RonSaveError),
    /// The workspace root search found no marker.
    NoWorkspaceRoot,
}

impl From<RonSaveError> for SavePrefabError {
    fn from(err: RonSaveError) -> Self {
        Self::Save(err)
    }
}

impl std::fmt::Display for SavePrefabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no prefab name entered — nothing to save"),
            Self::IllegalCell(slot) => {
                write!(
                    f,
                    "painted cell {slot:?} is an illegal placement; refusing to save"
                )
            }
            Self::Save(err) => write!(f, "{err}"),
            Self::NoWorkspaceRoot => write!(f, "no workspace root found — nowhere to save"),
        }
    }
}

impl std::error::Error for SavePrefabError {}

#[must_use]
pub(super) fn size_dir(size: GridSize) -> String {
    format!("{}x{}", *size.width(), *size.height())
}
