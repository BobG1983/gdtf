//! Classify terrain tiles for placement rules.

use gdtf_battle_sim::{
    level::ThemeUuid,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainPieceKind,
    },
};

/// Coarse class used by placement rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTileClass {
    /// Floor slab.
    Slab,
    /// Ladder (by display name).
    Ladder,
    /// Everything else.
    Other,
}

/// Whether a display name looks like a ladder.
#[must_use]
pub fn names_a_ladder(text: &str) -> bool {
    text.to_ascii_lowercase().contains("ladder")
}

/// Classify a terrain key from the registry.
#[must_use]
pub fn classify(
    registry: &TerrainDefRegistry,
    _theme: ThemeUuid,
    key: &TerrainUuid,
) -> EditorTileClass {
    let Some(def) = registry.def(key) else {
        return EditorTileClass::Other;
    };
    if names_a_ladder(&def.display_name) {
        return EditorTileClass::Ladder;
    }
    match def.sim_kind.kind() {
        TerrainPieceKind::Slab => EditorTileClass::Slab,
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => {
            EditorTileClass::Other
        }
    }
}
