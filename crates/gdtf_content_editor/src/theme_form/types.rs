//! Theme draft resource and save errors.

use bevy::prelude::*;
use gdtf_battle_sim::{level::ThemeUuid, terrain::def::TerrainUuid};

/// In-progress theme being authored.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct ThemeDraft {
    display_name: String,
    terrain: Vec<TerrainUuid>,
    default_floor: Option<TerrainUuid>,
    key: ThemeUuid,
}

impl ThemeDraft {
    /// Empty draft with a fresh key.
    #[must_use]
    pub fn new_theme() -> Self {
        Self {
            display_name: String::new(),
            terrain: Vec::new(),
            default_floor: None,
            key: ThemeUuid::generate(),
        }
    }

    /// Build from existing parts (e.g. loaded from registry).
    #[must_use]
    pub const fn from_parts(
        key: ThemeUuid,
        display_name: String,
        terrain: Vec<TerrainUuid>,
        default_floor: TerrainUuid,
    ) -> Self {
        Self {
            display_name,
            terrain,
            default_floor: Some(default_floor),
            key,
        }
    }

    /// Display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Set the display name.
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
    }

    /// Selected terrain keys.
    #[must_use]
    pub fn terrain(&self) -> &[TerrainUuid] {
        &self.terrain
    }

    /// Whether `terrain` is in the theme's list.
    #[must_use]
    pub fn has_terrain(&self, terrain: TerrainUuid) -> bool {
        self.terrain.contains(&terrain)
    }

    /// Add or remove a terrain key; clears default floor if removed.
    pub fn toggle_terrain(&mut self, terrain: TerrainUuid) {
        if let Some(pos) = self.terrain.iter().position(|t| *t == terrain) {
            self.terrain.remove(pos);
            if self.default_floor == Some(terrain) {
                self.default_floor = None;
            }
        } else {
            self.terrain.push(terrain);
        }
    }

    /// Default floor, if set.
    #[must_use]
    pub const fn default_floor(&self) -> Option<TerrainUuid> {
        self.default_floor
    }

    /// Set default floor when it is already in the terrain list.
    pub fn set_default_floor(&mut self, terrain: TerrainUuid) {
        if self.terrain.contains(&terrain) {
            self.default_floor = Some(terrain);
        }
    }

    /// Theme key.
    #[must_use]
    pub const fn key(&self) -> ThemeUuid {
        self.key
    }
}

impl Default for ThemeDraft {
    fn default() -> Self {
        Self::new_theme()
    }
}

/// Errors from theme validation or save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveThemeError {
    /// Display name is empty.
    EmptyName,
    /// No terrain selected.
    NoTerrain,
    /// Default floor is not in the terrain list.
    DefaultFloorNotInTerrain,
    /// Underlying RON write failed.
    Save(gdtf_assets::RonSaveError),
}

impl From<gdtf_assets::RonSaveError> for SaveThemeError {
    fn from(err: gdtf_assets::RonSaveError) -> Self {
        Self::Save(err)
    }
}

impl std::fmt::Display for SaveThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no theme name entered — nothing to save"),
            Self::NoTerrain => write!(
                f,
                "the theme has no terrain selected — nothing to draw from"
            ),
            Self::DefaultFloorNotInTerrain => {
                write!(
                    f,
                    "the default floor must be one of the theme's own terrain (C6)"
                )
            }
            Self::Save(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SaveThemeError {}
