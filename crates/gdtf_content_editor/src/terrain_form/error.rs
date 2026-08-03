#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTerrainError {
            EmptyName,
                    MissingMountedWeapon,
            Save(gdtf_assets::RonSaveError),
}

impl From<gdtf_assets::RonSaveError> for SaveTerrainError {
            fn from(err: gdtf_assets::RonSaveError) -> Self {
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
        }
    }
}

impl std::error::Error for SaveTerrainError {}
