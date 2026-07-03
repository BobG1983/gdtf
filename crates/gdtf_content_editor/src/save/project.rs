//! Pure **projection and serialization** functions — convert the in-memory [`EditorMap`] into
//! the [`PrefabSpec`] schema and serialize it to RON (GTW-432; swept onto the UUID schema
//! in GTW-495).
//!
//! No Bevy systems here — only pure data-transformation functions and their helpers. All
//! filesystem I/O lives in [`super::systems`].

use std::path::{Path, PathBuf};

use gdtf_assets::{FileStem, sanitize_file_stem, serialize_ron_pretty};
use gdtf_battle_sim::{
    level::{GridSize, PrefabSpec, TerrainPlacementEntry, ThemeUuid},
    terrain::def::TerrainDefRegistry,
};

use super::types::{
    MAPS_SUBDIR, PREFAB_EXTENSION, SAVED_SPAWN_ROLE, SavePrefabError, WORKSPACE_ASSETS_ROOT,
    size_dir, theme_dir,
};
use crate::{
    EditorMap,
    placement::{PlacementVerdict, ProposedPlacement, evaluate_placement},
    session::MapEditorSession,
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

/// The full on-disk PATH a saved prefab is written to:
/// `<workspace assets>/maps/<theme>/<size>/<stem>.prefab.ron` (GTW-432; root in GTW-495).
///
/// Pure (no IO) so a test can assert the resolved location without writing anything. The
/// `theme_display` is the slugified theme directory's source (the theme's display name), `size`
/// comes from the prefab being authored, `stem` is the sanitized prefab name.
#[must_use]
pub fn prefab_save_path(theme_display: &str, size: GridSize, stem: &str) -> PathBuf {
    Path::new(WORKSPACE_ASSETS_ROOT)
        .join(MAPS_SUBDIR)
        .join(theme_dir(theme_display))
        .join(size_dir(size))
        .join(format!("{stem}.{PREFAB_EXTENSION}"))
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
/// `assets/content/maps/<theme>/<size>/<stem>.prefab.ron` (GTW-515 C4.9 / C4.10), or return the typed
/// [`SavePrefabError`] (never a panic).
///
/// Sanitizes the entered prefab name to a file stem (the shared [`sanitize_file_stem`] seam),
/// projects the map to a [`PrefabSpec`] via [`editor_map_to_prefab`] (which re-checks every
/// painted cell through the shared [`evaluate_placement`] — C3 illegal-cell guard, reused
/// verbatim), resolves the themed/sized path via [`prefab_save_path`], and hands the
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
pub fn write_prefab(
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
    let path = prefab_save_path(theme_display, session.grid_size(), &stem);
    gdtf_assets::write_ron_pretty(&path, &spec)?;
    Ok(path)
}
