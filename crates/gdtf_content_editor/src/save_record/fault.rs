//! The union of what the editor's four save error types report.

use bevy::prelude::Deref;
use gdtf_assets::RonSaveError;
use gdtf_battle_sim::metric::CellLevel;

use crate::{terrain_form::SaveTerrainError, theme_form::SaveThemeError};

/// The message a serialize or write failure carried up from its source.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct SaveFaultMessage(String);

impl SaveFaultMessage {
    /// Wrap the message a failing save reported.
    #[must_use]
    pub const fn new(message: String) -> Self {
        Self(message)
    }
}

/// Why a save wrote no file, across every editor family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSaveFault {
    /// The name sanitized down to nothing, so there is no file stem.
    EmptyName,
    /// An emplacement terrain names no mounted weapon.
    MissingMountedWeapon,
    /// The theme selects no terrain to draw from.
    NoTerrain,
    /// The theme's default floor is not one of its own terrain.
    DefaultFloorNotInTerrain,
    /// A painted cell fails the editor's placement rules.
    IllegalCell(CellLevel),
    /// The workspace root search found no marker, so there is nowhere to write.
    NoWorkspaceRoot,
    /// serde/ron could not encode the value.
    Serialize(SaveFaultMessage),
    /// The filesystem write or directory create failed.
    Write(SaveFaultMessage),
}

impl From<RonSaveError> for EditorSaveFault {
    fn from(err: RonSaveError) -> Self {
        match err {
            RonSaveError::Serialize(detail) => Self::Serialize(SaveFaultMessage::new(detail)),
            RonSaveError::Write(detail) => Self::Write(SaveFaultMessage::new(detail)),
            RonSaveError::NoWorkspaceRoot => Self::NoWorkspaceRoot,
        }
    }
}

impl From<SaveTerrainError> for EditorSaveFault {
    fn from(err: SaveTerrainError) -> Self {
        match err {
            SaveTerrainError::EmptyName => Self::EmptyName,
            SaveTerrainError::MissingMountedWeapon => Self::MissingMountedWeapon,
            SaveTerrainError::Save(inner) => inner.into(),
            SaveTerrainError::NoWorkspaceRoot => Self::NoWorkspaceRoot,
        }
    }
}

impl From<SaveThemeError> for EditorSaveFault {
    fn from(err: SaveThemeError) -> Self {
        match err {
            SaveThemeError::EmptyName => Self::EmptyName,
            SaveThemeError::NoTerrain => Self::NoTerrain,
            SaveThemeError::DefaultFloorNotInTerrain => Self::DefaultFloorNotInTerrain,
            SaveThemeError::Save(inner) => inner.into(),
            SaveThemeError::NoWorkspaceRoot => Self::NoWorkspaceRoot,
        }
    }
}

#[cfg(debug_assertions)]
impl From<crate::save::SavePrefabError> for EditorSaveFault {
    fn from(err: crate::save::SavePrefabError) -> Self {
        match err {
            crate::save::SavePrefabError::EmptyName => Self::EmptyName,
            crate::save::SavePrefabError::IllegalCell(slot) => Self::IllegalCell(slot),
            crate::save::SavePrefabError::Save(inner) => inner.into(),
            crate::save::SavePrefabError::NoWorkspaceRoot => Self::NoWorkspaceRoot,
        }
    }
}
