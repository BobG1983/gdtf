//! Pure **projection and serialization** functions — convert the in-memory [`EditorMap`] into
//! the [`PrefabSpec`] schema and serialize it to RON (GTW-432; swept onto the UUID schema
//! in GTW-495).
//!
//! No Bevy systems here — only pure data-transformation functions and their helpers. The
//! filesystem write funnels through the root-parameterized [`write_prefab_in`] core (GTW-662)
//! and its thin production wrapper [`write_prefab`], both debug-only.

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

/// Sanitize the entered prefab name into a file-name STEM (GTW-432) — since GTW-577 a thin
/// delegation to the shared [`sanitize_file_stem`] seam (trimmed, lowercased, separators →
/// underscores, anything outside `[a-z0-9_]` dropped).
///
/// Returns an empty [`FileStem`] for a name that sanitizes to nothing, which the caller
/// treats as [`SavePrefabError::EmptyName`].
#[must_use]
pub fn sanitize_name(raw: &str) -> FileStem {
    sanitize_file_stem(raw)
}

/// The full on-disk PATH a saved prefab is written to under an arbitrary assets `root`:
/// `<root>/content/maps/<theme>/<size>/<stem>.prefab.ron` — the root-parameterized core
/// (GTW-662, the GTW-555 pattern), so a test resolves the REAL save location against a
/// `TempDir` root instead of the version-controlled `assets/` tree.
///
/// Pure (no IO) so a test can assert the resolved location without writing anything. The
/// `theme_display` is the slugified theme directory's source (the theme's display name), `size`
/// comes from the prefab being authored, `stem` is the sanitized prefab name.
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

/// The full on-disk PATH a saved prefab is written to:
/// `<workspace assets>/maps/<theme>/<size>/<stem>.prefab.ron` (GTW-432; root in GTW-495).
///
/// Thin wrapper around [`prefab_save_path_in`] that supplies the workspace `assets/` root
/// ([`WORKSPACE_ASSETS_ROOT`]) — the location the production [`write_prefab`] writes to.
#[must_use]
pub fn prefab_save_path(theme_display: &str, size: GridSize, stem: &str) -> PathBuf {
    prefab_save_path_in(Path::new(WORKSPACE_ASSETS_ROOT), theme_display, size, stem)
}

/// Project the in-memory [`EditorMap`] (incl. multi-level cells) into the [`PrefabSpec`]
/// schema — the conversion at the heart of the save (C2 / C3).
///
/// Iterates every painted `(slot, tile)` of the map, re-checks each through the GTW-430 shared
/// [`evaluate_placement`] predicate (C3 — an illegal cell rejects the whole save), and collapses
/// them into ONE [`placements`](PrefabSpec::placements) list of [`TerrainPlacementEntry`] (the
/// schema's single list — the per-piece behaviour now lives in the referenced
/// [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)). The prefab's theme / size come from
/// the `session`; the spawn role is the [`SAVED_SPAWN_ROLE`] default. There is NO authored-opening
/// derivation — the schema carries none (connectivity is by-construction in the assembler:
/// the 1-cell `default_floor` seam every placement reserves).
///
/// # Errors
///
/// [`SavePrefabError::IllegalCell`] if any painted cell is an illegal placement.
pub fn editor_map_to_prefab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
) -> Result<PrefabSpec, SavePrefabError> {
    let theme: ThemeUuid = session.theme();
    let size = session.grid_size();

    let mut placements: Vec<TerrainPlacementEntry> = Vec::new();
    for (slot, tile) in map.painted() {
        // C3: re-evaluate every painted cell through the shared GTW-430 predicate; an illegal
        // cell rejects the whole save (a saved prefab never contains an illegal cell).
        let placement = ProposedPlacement::new(*slot, *tile);
        if let PlacementVerdict::Illegal(_) =
            evaluate_placement(map, registry, theme, &placement, size)
        {
            return Err(SavePrefabError::IllegalCell(*slot));
        }
        placements.push(TerrainPlacementEntry::new(*tile, *slot));
    }

    // Order-stable so the serialized output is deterministic across runs (the model iterates a
    // HashMap, so sort by (slot, piece)).
    placements.sort_by(|a, b| {
        (a.at.x, a.at.y, a.at.z, *a.piece).cmp(&(b.at.x, b.at.y, b.at.z, *b.piece))
    });

    Ok(PrefabSpec::new(theme, size, SAVED_SPAWN_ROLE, placements))
}

/// Serialize a built [`PrefabSpec`] to its `.prefab.ron`-shaped RON text — the SAME schema
/// the GTW-489 loader deserializes (C2). Delegates to the shared
/// [`serialize_ron_pretty`] seam (GTW-577 C2), so a saved prefab stays human-editable like
/// the shipped `assets/content/maps/**/*.prefab.ron`.
///
/// # Errors
///
/// [`SavePrefabError::Save`] wrapping the seam's serialize failure.
pub fn serialize_prefab(spec: &PrefabSpec) -> Result<String, SavePrefabError> {
    serialize_ron_pretty(spec).map_err(SavePrefabError::Save)
}

/// Project + serialize + WRITE the [`EditorMap`] to
/// `<assets_root>/content/maps/<theme>/<size>/<stem>.prefab.ron` (GTW-515 C4.9 / C4.10), or
/// return the typed [`SavePrefabError`] (never a panic).
///
/// This is the **root-parameterized core** (GTW-662 — the GTW-555 `write_terrain_in`
/// precedent): all path-building, serialization, and `fs` writes go through here.
/// `assets_root` is the on-disk parent of the `content/maps/` subtree: production passes
/// [`WORKSPACE_ASSETS_ROOT`] (via [`write_prefab`]); tests pass a unique `tempfile::TempDir`
/// root so no test ever writes into the version-controlled `assets/` tree.
///
/// Sanitizes the entered prefab name to a file stem (the shared [`sanitize_file_stem`] seam),
/// projects the map to a [`PrefabSpec`] via [`editor_map_to_prefab`] (which re-checks every
/// painted cell through the shared [`evaluate_placement`] — C3 illegal-cell guard, reused
/// verbatim), resolves the themed/sized path via [`prefab_save_path_in`], and hands the
/// serialize → mkdir → write chain to the shared
/// [`write_ron_pretty`](gdtf_assets::write_ron_pretty) seam (GTW-577 C2). Returns the resolved
/// [`PathBuf`] on success so the caller can log it. Debug-only — the whole save path is gated
/// `#[cfg(debug_assertions)]` (the GTW-429 gang-save precedent), so it never compiles into a
/// release binary.
///
/// The pure projection/serialize halves ([`editor_map_to_prefab`] / [`serialize_prefab`]) are the
/// SAME ones the in-crate round-trip test exercises; this wraps them with the name-validation + the
/// filesystem write.
///
/// # Errors
///
/// [`SavePrefabError::EmptyName`] if the sanitized name is empty; [`SavePrefabError::IllegalCell`]
/// if a painted cell is illegal; [`SavePrefabError::Save`] from the seam's serialization / file
/// write.
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

/// Project + serialize + WRITE the [`EditorMap`] to
/// `assets/content/maps/<theme>/<size>/<stem>.prefab.ron` (GTW-515 C4.9 / C4.10), or return the
/// typed [`SavePrefabError`] (never a panic).
///
/// Thin wrapper around [`write_prefab_in`] that supplies the workspace `assets/` root
/// ([`WORKSPACE_ASSETS_ROOT`]) — byte-identical paths for production callers (GTW-662 C3).
/// This is the function the egui "Save prefab" button calls; the file lands exactly where the
/// GTW-489 folder loader reads from.
///
/// # Errors
///
/// Any [`SavePrefabError`] from name validation, the projection's illegal-cell guard, or the
/// seam's serialization / file write (see [`write_prefab_in`]).
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
