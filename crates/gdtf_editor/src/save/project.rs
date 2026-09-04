//! Prefab conversion and RON save helpers.

use std::path::{Path, PathBuf};

use cobalt_ron_assets::{
    FileStem, sanitize_file_stem, serialize_ron_pretty, workspace_assets_root,
};
use gdtf_battle_sim::{
    level::{GridSize, PrefabSpec, TerrainPlacementEntry, ThemeUuid},
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_families::prefabs::{PREFAB_EXTENSION, PREFABS_FOLDER};

use super::types::{SAVED_SPAWN_ROLE, SavePrefabError, size_dir};
use crate::{
    EditorMap,
    placement::{PlacementVerdict, ProposedPlacement, evaluate_placement},
    session::MapEditorSession,
    theme_dir::theme_dir,
};

/// Sanitize a user-entered prefab name into a file stem.
#[must_use]
pub fn sanitize_name(raw: &str) -> FileStem {
    sanitize_file_stem(raw)
}

/// Full path for a prefab under `root`.
#[must_use]
pub fn prefab_save_path_in(
    root: &Path,
    theme_display: &str,
    size: GridSize,
    stem: &str,
) -> PathBuf {
    root.join(PREFABS_FOLDER)
        .join(theme_dir(theme_display))
        .join(size_dir(size))
        .join(format!("{stem}.{PREFAB_EXTENSION}"))
}

/// Full path for a prefab under the workspace assets root, or `None` when the search finds no root.
#[must_use]
pub fn prefab_save_path(theme_display: &str, size: GridSize, stem: &str) -> Option<PathBuf> {
    workspace_assets_root().map(|root| prefab_save_path_in(&root, theme_display, size, stem))
}

/// Build a prefab spec from the painted map and session.
///
/// # Errors
///
/// Returns [`SavePrefabError::IllegalCell`] when a painted cell fails placement rules.
pub fn editor_map_to_prefab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
) -> Result<PrefabSpec, SavePrefabError> {
    let theme: ThemeUuid = session.theme();
    let size = session.grid_size();

    let mut placements: Vec<TerrainPlacementEntry> = Vec::new();
    for (slot, piece) in map.painted() {
        let placement = ProposedPlacement::new(*slot, piece.tile(), piece.facing());
        if let PlacementVerdict::Illegal(_) =
            evaluate_placement(map, registry, theme, &placement, size)
        {
            return Err(SavePrefabError::IllegalCell(*slot));
        }
        placements.push(TerrainPlacementEntry::new(
            piece.tile(),
            *slot,
            piece.facing(),
        ));
    }

    placements.sort_by(|a, b| {
        (a.at.x, a.at.y, a.at.z, *a.piece).cmp(&(b.at.x, b.at.y, b.at.z, *b.piece))
    });

    Ok(PrefabSpec::new(theme, size, SAVED_SPAWN_ROLE, placements))
}

/// Serialize a prefab spec to pretty RON.
///
/// # Errors
///
/// Returns [`SavePrefabError::Save`] if serialization fails.
pub fn serialize_prefab(spec: &PrefabSpec) -> Result<String, SavePrefabError> {
    serialize_ron_pretty(spec).map_err(SavePrefabError::Save)
}

/// Write prefab RON under `assets_root`.
///
/// # Errors
///
/// Returns [`SavePrefabError`] on empty name, illegal cells, or write failure.
#[cfg(feature = "mcp")]
pub fn write_prefab_in(
    assets_root: &Path,
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
    theme_display: &str,
    raw_name: &str,
) -> Result<PathBuf, SavePrefabError> {
    let stem = sanitize_name(raw_name);
    if stem.is_empty() {
        return Err(SavePrefabError::EmptyName);
    }
    let spec = editor_map_to_prefab(map, registry, session)?;
    let path = prefab_save_path_in(assets_root, theme_display, session.grid_size(), &stem);
    cobalt_ron_assets::write_ron_pretty(&path, &spec)?;
    Ok(path)
}

/// Write prefab RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`SavePrefabError`] on a failed root search, empty name, illegal cells, or write failure.
#[cfg(feature = "mcp")]
pub fn write_prefab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
    theme_display: &str,
    raw_name: &str,
) -> Result<PathBuf, SavePrefabError> {
    let Some(root) = workspace_assets_root() else {
        return Err(SavePrefabError::NoWorkspaceRoot);
    };
    write_prefab_in(&root, map, registry, session, theme_display, raw_name)
}
