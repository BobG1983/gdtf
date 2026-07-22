//! The TERRAIN form's **typed save failure** (GTW-474; split out of `types.rs` in
//! GTW-574): the [`SaveTerrainError`] the projection + save path return instead of
//! panicking.

/// Why a terrain save was REJECTED — the handled, no-panic failure of the terrain save path
/// (GTW-474). A named domain enum (no-bare-types). `pub` because the `pub`
/// [`serialize_terrain_def`](crate::serialize_terrain_def) returns it (the C4 test reuses the
/// projection + serialization path). The domain-validation variants stay bespoke (GTW-577
/// P9); the serialize/write tail collapsed onto the shared
/// [`RonSaveError`](gdtf_assets::RonSaveError), wrapped by [`Save`](Self::Save).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTerrainError {
    /// The author entered no display name (an empty / whitespace-only field) — there is no file
    /// stem to write to.
    EmptyName,
    /// The draft's kind is Emplacement but NO mounted weapon is selected (GTW-574 C6) —
    /// an [`TerrainSimKind::Emplacement`](gdtf_battle_sim::terrain::def::TerrainSimKind::Emplacement)
    /// def REQUIRES its mounted-weapon registry key, so the projection FAILS CLOSED:
    /// no panic, no silent default weapon, nothing written.
    MissingMountedWeapon,
    /// The shared serialize/write tail failed (GTW-577 C3) — wraps the shared writer's
    /// [`RonSaveError`](gdtf_assets::RonSaveError), whose `Display` names the failed stage.
    Save(gdtf_assets::RonSaveError),
}

impl From<gdtf_assets::RonSaveError> for SaveTerrainError {
    /// The per-type conversion off the shared writer error (GTW-577 C3) — lets the save path
    /// `?` a write failure straight into the form's error.
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
