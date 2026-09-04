//! Terrain save errors.

/// Errors from terrain validation or save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTerrainError {
    /// Display name is empty.
    EmptyName,
    /// Emplacement kind without a mounted weapon.
    MissingMountedWeapon,
    /// Underlying RON write failed.
    Save(cobalt_ron_assets::RonSaveError),
    /// The workspace root search found no marker.
    NoWorkspaceRoot,
}

impl From<cobalt_ron_assets::RonSaveError> for SaveTerrainError {
    fn from(err: cobalt_ron_assets::RonSaveError) -> Self {
        Self::Save(err)
    }
}

impl std::fmt::Display for SaveTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no terrain name entered — nothing to save"),
            Self::MissingMountedWeapon => write!(
                f,
                "no mounted weapon selected — an Emplacement terrain requires one"
            ),
            Self::Save(err) => write!(f, "{err}"),
            Self::NoWorkspaceRoot => write!(f, "no workspace root found — nowhere to save"),
        }
    }
}

impl std::error::Error for SaveTerrainError {}
