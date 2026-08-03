use std::path::{Path, PathBuf};

use gdtf_assets::{FileStem, WORKSPACE_ASSETS_ROOT, sanitize_file_stem, serialize_ron_pretty};
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

#[must_use]
pub fn sanitize_name(raw: &str) -> FileStem {
    sanitize_file_stem(raw)
}

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

#[must_use]
pub fn prefab_save_path(theme_display: &str, size: GridSize, stem: &str) -> PathBuf {
    prefab_save_path_in(Path::new(WORKSPACE_ASSETS_ROOT), theme_display, size, stem)
}

pub fn editor_map_to_prefab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
) -> Result<PrefabSpec, SavePrefabError> {
    let theme: ThemeUuid = session.theme();
    let size = session.grid_size();

    let mut placements: Vec<TerrainPlacementEntry> = Vec::new();
    for (slot, tile) in map.painted() {
        let placement = ProposedPlacement::new(*slot, *tile);
        if let PlacementVerdict::Illegal(_) =
            evaluate_placement(map, registry, theme, &placement, size)
        {
            return Err(SavePrefabError::IllegalCell(*slot));
        }
        placements.push(TerrainPlacementEntry::new(*tile, *slot));
    }

    placements.sort_by(|a, b| {
        (a.at.x, a.at.y, a.at.z, *a.piece).cmp(&(b.at.x, b.at.y, b.at.z, *b.piece))
    });

    Ok(PrefabSpec::new(theme, size, SAVED_SPAWN_ROLE, placements))
}

pub fn serialize_prefab(spec: &PrefabSpec) -> Result<String, SavePrefabError> {
    serialize_ron_pretty(spec).map_err(SavePrefabError::Save)
}

/// `#[cfg(debug_assertions)]` (the GTW-429 gang-save precedent), so it never compiles into a
#[cfg(debug_assertions)]
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
    gdtf_assets::write_ron_pretty(&path, &spec)?;
    Ok(path)
}

#[cfg(debug_assertions)]
pub fn write_prefab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
    theme_display: &str,
    raw_name: &str,
) -> Result<PathBuf, SavePrefabError> {
    write_prefab_in(
        Path::new(WORKSPACE_ASSETS_ROOT),
        map,
        registry,
        session,
        theme_display,
        raw_name,
    )
}
