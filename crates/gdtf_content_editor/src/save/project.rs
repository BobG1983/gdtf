//! Pure **projection and serialization** functions — convert the in-memory [`EditorMap`] into
//! the v2 [`PrefabSpecV2`] schema and serialize it to RON (GTW-432; swept onto the v2 UUID schema
//! in GTW-495).
//!
//! No Bevy systems here — only pure data-transformation functions and their helpers. All
//! filesystem I/O lives in [`super::systems`].

use std::path::{Path, PathBuf};

use gdtf_battle_sim::{
    level::{GridSize, PrefabSpecV2, TerrainPlacementEntry, ThemeUuid},
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

/// Sanitize the entered prefab name into a file-name STEM — trimmed, lowercased, spaces →
/// underscores, and any character outside `[a-z0-9_]` dropped (GTW-432).
///
/// Returns the empty string for a name that sanitizes to nothing, which the caller treats as
/// [`SavePrefabError::EmptyName`].
#[must_use]
pub fn sanitize_name(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// The full on-disk PATH a saved prefab is written to:
/// `<workspace assets>/maps/<theme>/<size>/<stem>.prefab_v2.ron` (GTW-432; v2 root in GTW-495).
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

/// Project the in-memory [`EditorMap`] (incl. multi-level cells) into the v2 [`PrefabSpecV2`]
/// schema — the conversion at the heart of the save (C2 / C3).
///
/// Iterates every painted `(slot, tile)` of the map, re-checks each through the GTW-430 shared
/// [`evaluate_placement`] predicate (C3 — an illegal cell rejects the whole save), and collapses
/// them into ONE [`placements`](PrefabSpecV2::placements) list of [`TerrainPlacementEntry`] (the
/// v2 schema's single list — the per-piece behaviour now lives in the referenced
/// [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)). The prefab's theme / size come from
/// the `session`; the spawn role is the [`SAVED_SPAWN_ROLE`] default. There is NO authored-opening
/// derivation — the v2 schema carries none (connectivity is by-construction in the v2 assembler:
/// the 1-cell `default_floor` seam every placement reserves).
///
/// # Errors
///
/// [`SavePrefabError::IllegalCell`] if any painted cell is an illegal placement.
pub fn editor_map_to_prefab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    session: &MapEditorSession,
) -> Result<PrefabSpecV2, SavePrefabError> {
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

    Ok(PrefabSpecV2::new(theme, size, SAVED_SPAWN_ROLE, placements))
}

/// Serialize a built [`PrefabSpecV2`] to its `.prefab_v2.ron`-shaped RON text — the SAME schema
/// the GTW-489 loader deserializes (C2). A pretty-printed record so a saved prefab stays
/// human-editable like the shipped `assets/maps/**/*.prefab_v2.ron`.
///
/// # Errors
///
/// [`SavePrefabError::Serialize`] wrapping the underlying RON serialization error.
pub fn serialize_prefab(spec: &PrefabSpecV2) -> Result<String, SavePrefabError> {
    ron::ser::to_string_pretty(spec, ron::ser::PrettyConfig::default())
        .map_err(|err| SavePrefabError::Serialize(err.to_string()))
}

/// Project + serialize + WRITE the [`EditorMap`] to
/// `assets/maps/<theme>/<size>/<stem>.prefab_v2.ron` (GTW-515 C4.9 / C4.10), or return the typed
/// [`SavePrefabError`] (never a panic).
///
/// Sanitizes the entered prefab name to a file stem, projects the map to a [`PrefabSpecV2`] via
/// [`editor_map_to_prefab`] (which re-checks every painted cell through the shared
/// [`evaluate_placement`] — C3 illegal-cell guard, reused verbatim), serializes it via
/// [`serialize_prefab`], resolves the themed/sized path via [`prefab_save_path`], creates the
/// directory if absent, and writes the file. Returns the resolved [`PathBuf`] on success so the
/// caller can log it. Debug-only — the whole save path is gated `#[cfg(debug_assertions)]` (the
/// GTW-429 gang-save precedent), so it never compiles into a release binary.
///
/// The pure projection/serialize halves ([`editor_map_to_prefab`] / [`serialize_prefab`]) are the
/// SAME ones the in-crate round-trip test exercises; this wraps them with the name-validation + the
/// filesystem write.
///
/// # Errors
///
/// [`SavePrefabError::EmptyName`] if the sanitized name is empty; [`SavePrefabError::IllegalCell`]
/// if a painted cell is illegal; [`SavePrefabError::Serialize`] / [`SavePrefabError::Write`] from
/// serialization / the file write.
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
    let serialized = serialize_prefab(&spec)?;
    let path = prefab_save_path(theme_display, session.grid_size(), &stem);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| SavePrefabError::Write(err.to_string()))?;
    }
    std::fs::write(&path, serialized).map_err(|err| SavePrefabError::Write(err.to_string()))?;
    Ok(path)
}
