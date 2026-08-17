//! Theme draft resource and save errors.

use bevy::prelude::*;
use gdtf_battle_sim::{level::ThemeUuid, terrain::def::TerrainUuid};

/// In-progress theme being authored.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct ThemeDraft {
    display_name:   String,
    terrain:        Vec<TerrainUuid>,
    default_floor:  Option<TerrainUuid>,
    key:            ThemeUuid,
    user_blank:     bool,
    pinned_session: Option<ThemeUuid>,
}

impl ThemeDraft {
    fn blank(user_blank: bool) -> Self {
        Self {
            display_name: String::new(),
            terrain: Vec::new(),
            default_floor: None,
            key: ThemeUuid::generate(),
            user_blank,
            pinned_session: None,
        }
    }

    /// Empty draft the user just asked to author — must not be overwritten by sync.
    #[must_use]
    pub fn new_theme() -> Self {
        Self::blank(true)
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
            user_blank: false,
            pinned_session: None,
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

    /// Whether this draft is a user-started blank that sync must keep.
    #[must_use]
    pub const fn user_blank(&self) -> bool {
        self.user_blank
    }

    /// Session theme this blank was pinned against, if any.
    #[must_use]
    pub const fn pinned_session(&self) -> Option<ThemeUuid> {
        self.pinned_session
    }

    /// Remember the session theme that must not overwrite this blank.
    pub const fn pin_session(&mut self, session: ThemeUuid) {
        self.pinned_session = Some(session);
    }
}

impl Default for ThemeDraft {
    fn default() -> Self {
        Self::blank(false)
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
    /// The workspace root search found no marker.
    NoWorkspaceRoot,
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
            Self::NoWorkspaceRoot => write!(f, "no workspace root found — nowhere to save"),
        }
    }
}

impl std::error::Error for SaveThemeError {}
