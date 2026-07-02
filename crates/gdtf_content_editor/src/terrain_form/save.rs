//! The TERRAIN-mode form's **projection + serialization + write** (GTW-474): turn the in-progress
//! [`TerrainDraft`] into a real [`TerrainDef`], serialize it to a `.terrain_def.ron`, and write it
//! to the active theme's terrain folder so the GTW-487 terrain loader resolves it into the
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry).
//!
//! [`draft_to_terrain_def`] + [`serialize_terrain_def`] are PURE (no IO) so a test can round-trip
//! them without touching the assets tree (the C3/C4 round-trip). The filesystem write lives in
//! [`write_terrain_in`] (root-parameterized core, debug-only) and its thin [`write_terrain`] wrapper
//! (the GTW-432 save precedent). Tests call `write_terrain_in` with a unique `tempfile::TempDir` root
//! so they never pollute the version-controlled `assets/` tree.

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainUuid},
    piece::TerrainGraphicKey,
};

use super::types::{SaveTerrainError, TerrainDraft, TerrainKindChoice};

/// The workspace `assets/` root — byte-identical to the editor's `AssetPlugin.file_path`
/// (`crates/gdtf_content_editor` → up two levels → `assets`), computed at compile time. So a
/// terrain def the editor SAVES lands exactly where the GTW-487 terrain loader READS from —
/// `assets/terrain/<theme>/`.
#[cfg(debug_assertions)]
const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The per-theme terrain root the GTW-487 loader scans — `assets/terrain/<theme>/`. The SOLE
/// terrain root after GTW-490.
#[cfg(debug_assertions)]
const TERRAIN_SUBDIR: &str = "terrain";

/// The compound file extension the GTW-487 terrain loader keys on
/// (`init_ron_asset_with_extensions::<TerrainDef>(vec!["terrain_def.ron"])`) — a saved def MUST
/// use it or the loader never picks the file up.
#[cfg(debug_assertions)]
const TERRAIN_DEF_EXTENSION: &str = "terrain_def.ron";

/// Sanitize the entered display name into a file-name STEM — trimmed, lowercased, spaces /
/// dashes → underscores, anything outside `[a-z0-9_]` dropped (the prefab `sanitize_name`
/// sibling).
///
/// Returns the empty string for a name that sanitizes to nothing (the caller treats it as
/// [`SaveTerrainError::EmptyName`]).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn sanitize_stem(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// The `snake_case` directory name for a theme, derived from its human label — the prefab
/// `theme_dir` sibling. `assets/terrain/<theme>/` keys on the slugified theme DISPLAY NAME
/// (`"Industrial Hive"` → `industrial_hive`), matching the shipped per-theme layout.
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn theme_dir(display_name: &str) -> String {
    let slug: String = display_name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if slug.is_empty() {
        "unknown_theme".to_owned()
    } else {
        slug
    }
}

/// Project the in-progress [`TerrainDraft`] into a real [`TerrainDef`] keyed by `uuid` (GTW-474
/// C2) — the conversion at the heart of the terrain save.
///
/// Builds the [`sim_kind`](TerrainDef::sim_kind) from the chosen [`TerrainKindChoice`] (the right
/// stat fields per kind — Wall / Cover carry HP + armor + band; Slab carries HP + armor only),
/// the [`presenter_kind`](TerrainDef::presenter_kind) from the chosen graphic role + (Slab-only,
/// C2) footfall, and copies the selected tags. Pure — the caller supplies the (already-minted)
/// `uuid`.
///
/// `pub` (via the crate's [`draft_to_terrain_def`](crate::draft_to_terrain_def) re-export) so the
/// C4 integration test can project the live draft through the SAME conversion the save button runs
/// and assert the produced def round-trips through the GTW-487 loader into a
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry).
#[must_use]
pub fn draft_to_terrain_def(draft: &TerrainDraft, uuid: TerrainUuid) -> TerrainDef {
    let graphic_name: TerrainGraphicKey = draft.graphic().graphic_key();
    let (sim_kind, presenter_kind) = match draft.kind() {
        TerrainKindChoice::Wall => (
            TerrainSimKind::Wall {
                hp:               draft.cover_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
                height_band:      draft.height_band(),
            },
            TerrainPresenterKind::Wall { graphic_name },
        ),
        TerrainKindChoice::Cover => (
            TerrainSimKind::Cover {
                hp:               draft.cover_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
                height_band:      draft.height_band(),
            },
            TerrainPresenterKind::Cover { graphic_name },
        ),
        TerrainKindChoice::Slab => (
            TerrainSimKind::Slab {
                hp:               draft.slab_hp(),
                armor_protection: draft.armor_protection(),
                armor_hardness:   draft.armor_hardness(),
            },
            // C2: footfall is OFFERED only for Slab — the draft forces `None` for any other kind,
            // and `FootfallChoice::footfall` maps `None` → `None`, so a non-slab def never carries
            // a footfall (fail-closed re-validated here by construction).
            TerrainPresenterKind::Slab {
                graphic_name,
                footfall: draft.footfall().footfall(),
            },
        ),
    };
    TerrainDef {
        key: uuid,
        display_name: TerrainDisplayName::new(draft.display_name().trim().to_owned()),
        sim_kind,
        presenter_kind,
        tags: draft.tags().to_vec(),
        on_death: None,
    }
}

/// Serialize a built [`TerrainDef`] to its `.terrain_def.ron`-shaped RON text — the SAME schema
/// the GTW-487 terrain loader deserializes (C3). Pretty-printed so a saved def stays
/// human-editable like the shipped `assets/terrain/**/*.terrain_def.ron`.
///
/// # Errors
///
/// [`SaveTerrainError::Serialize`] wrapping the underlying RON serialization error.
pub fn serialize_terrain_def(def: &TerrainDef) -> Result<String, SaveTerrainError> {
    ron::ser::to_string_pretty(def, ron::ser::PrettyConfig::default())
        .map_err(|err| SaveTerrainError::Serialize(err.to_string()))
}

/// Build + serialize + WRITE a terrain def to `<assets_root>/terrain/<theme>/<stem>.terrain_def.ron`,
/// or return the typed [`SaveTerrainError`] (never a panic).
///
/// This is the **root-parameterized core** — all path-building, serialization, and `fs` writes go
/// through here. `assets_root` is the on-disk parent of the `terrain/` subtree: production passes
/// [`WORKSPACE_ASSETS_ROOT`] (via [`write_terrain`]); tests pass a unique `tempfile::TempDir` root
/// so no test ever writes into the version-controlled `assets/` tree.
///
/// Sanitizes the entered name to a file stem, projects the draft to a [`TerrainDef`] keyed by
/// `uuid`, serializes it, creates the themed directory if absent, and writes the file. Returns the
/// resolved [`PathBuf`] on success so the caller can log it or read it back.
///
/// # Errors
///
/// Any [`SaveTerrainError`] from name validation, serialization, or the file write.
#[cfg(debug_assertions)]
pub fn write_terrain_in(
    assets_root: &Path,
    draft: &TerrainDraft,
    uuid: TerrainUuid,
    theme_display: &str,
) -> Result<PathBuf, SaveTerrainError> {
    let stem = sanitize_stem(draft.display_name());
    if stem.is_empty() {
        return Err(SaveTerrainError::EmptyName);
    }
    let def = draft_to_terrain_def(draft, uuid);
    let serialized = serialize_terrain_def(&def)?;
    let path = assets_root
        .join(TERRAIN_SUBDIR)
        .join(theme_dir(theme_display))
        .join(format!("{stem}.{TERRAIN_DEF_EXTENSION}"));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| SaveTerrainError::Write(err.to_string()))?;
    }
    std::fs::write(&path, serialized).map_err(|err| SaveTerrainError::Write(err.to_string()))?;
    Ok(path)
}

/// Build + serialize + WRITE a terrain def to `assets/terrain/<theme>/<stem>.terrain_def.ron`
/// (GTW-474 C3), or return the typed [`SaveTerrainError`] (never a panic).
///
/// Thin wrapper around [`write_terrain_in`] that supplies the workspace `assets/` root
/// ([`WORKSPACE_ASSETS_ROOT`]). This is the function the egui Save button calls; the production
/// write path is UNCHANGED — the file lands exactly where the GTW-487 terrain loader reads from.
///
/// # Errors
///
/// Any [`SaveTerrainError`] from name validation, serialization, or the file write.
#[cfg(debug_assertions)]
pub fn write_terrain(
    draft: &TerrainDraft,
    uuid: TerrainUuid,
    theme_display: &str,
) -> Result<PathBuf, SaveTerrainError> {
    write_terrain_in(Path::new(WORKSPACE_ASSETS_ROOT), draft, uuid, theme_display)
}
