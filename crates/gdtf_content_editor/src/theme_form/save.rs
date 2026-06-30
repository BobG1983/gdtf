//! The THEME-mode form's **projection + serialization + write** (GTW-475): turn the in-progress
//! [`ThemeDraft`] into a real [`UuidThemeDef`], serialize it to a `.terrain_theme.ron`, and write
//! it to `assets/terrain/<slug>/<slug>.terrain_theme.ron` so the GTW-487 theme loader
//! (`resolve_theme_defs`) resolves it into the
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry).
//!
//! [`draft_to_theme_def`] + [`serialize_theme_def`] + [`theme_save_path`] are PURE (no IO) so a
//! test can round-trip them without touching the assets tree (the C7 round-trip). The filesystem
//! write lives in [`write_theme`] (debug-only — the whole `theme_form` save trigger is gated, the
//! GTW-474 terrain-save precedent).

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gdtf_battle_sim::level::{ThemeDisplayName, ThemeUuid, UuidThemeDef};

use super::types::{SaveThemeError, ThemeDraft};

/// The workspace `assets/` root — byte-identical to the editor's `AssetPlugin.file_path`
/// (`crates/gdtf_content_editor` → up two levels → `assets`), computed at compile time. So a
/// theme def the editor SAVES lands exactly where the GTW-487 theme loader READS from —
/// `assets/terrain/<slug>/`.
#[cfg(debug_assertions)]
const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The per-theme terrain root the GTW-487 loader scans — `assets/terrain/<slug>/`. The SOLE
/// terrain/theme root after GTW-490, shared with the per-theme `.terrain_def.ron` files.
#[cfg(debug_assertions)]
const TERRAIN_SUBDIR: &str = "terrain";

/// The compound file extension the GTW-487 theme loader keys on — a saved theme MUST use it or
/// the loader never picks the file up (the filename STEM is cosmetic; the `key` field is the
/// UUID).
#[cfg(debug_assertions)]
const THEME_DEF_EXTENSION: &str = "terrain_theme.ron";

/// The `snake_case` slug for a theme, derived from its human display name — the TERRAIN form's
/// `theme_dir` sibling. Both the per-theme DIRECTORY and the file STEM key on this slug
/// (`"Industrial Hive"` → `industrial_hive`), matching the shipped per-theme layout
/// (`assets/terrain/underhive/underhive.terrain_theme.ron`).
///
/// Returns the empty string for a name that slugifies to nothing (the caller treats it as
/// [`SaveThemeError::EmptyName`]).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn slugify(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// The full on-disk PATH a saved theme def is written to:
/// `<workspace assets>/terrain/<slug>/<slug>.terrain_theme.ron` (GTW-475 C5).
///
/// Pure (no IO) so a test can assert the resolved location without writing. `slug` is the
/// slugified display name (the same value names both the dir and the file stem).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn theme_save_path(slug: &str) -> PathBuf {
    Path::new(WORKSPACE_ASSETS_ROOT)
        .join(TERRAIN_SUBDIR)
        .join(slug)
        .join(format!("{slug}.{THEME_DEF_EXTENSION}"))
}

/// Project the in-progress [`ThemeDraft`] into a real [`UuidThemeDef`] keyed by `key` (GTW-475
/// C2/C5) — the conversion at the heart of the theme save.
///
/// Copies the selected terrain palette and the chosen default floor BY REFERENCE (UUIDs only —
/// the theme never inlines terrain stats, C3). Pure — the caller supplies the (already-minted /
/// loaded) `key`. This is intentionally LENIENT (it does not validate the default-floor-in-
/// palette rule) so the live RON PREVIEW can render an in-progress draft; the C6 validation is
/// enforced by [`validate_for_save`] on the save path. When the draft has no default floor yet,
/// the projected def carries the [`TerrainUuid::nil`](gdtf_battle_sim::terrain::def::TerrainUuid::nil)
/// sentinel (a defensible placeholder for the preview only — never written by a save, which
/// rejects a no-floor draft).
///
/// `pub` (via the crate's [`draft_to_theme_def`](crate::draft_to_theme_def) re-export) so the C7
/// tests + the C-integration test can project the live draft through the SAME conversion the
/// save button runs and assert the produced def round-trips through the GTW-487 loader's parser.
#[must_use]
pub fn draft_to_theme_def(draft: &ThemeDraft, key: ThemeUuid) -> UuidThemeDef {
    UuidThemeDef {
        key,
        display_name: ThemeDisplayName::new(draft.display_name().trim().to_owned()),
        default_floor: draft
            .default_floor()
            .unwrap_or_else(gdtf_battle_sim::terrain::def::TerrainUuid::nil),
        terrain: draft.terrain().to_vec(),
    }
}

/// Serialize a built [`UuidThemeDef`] to its `.terrain_theme.ron`-shaped RON text — the SAME
/// schema the GTW-487 theme loader (`resolve_theme_defs`) deserializes (C3/C5). Pretty-printed so
/// a saved def stays human-editable like the shipped `assets/terrain/**/*.terrain_theme.ron`.
///
/// # Errors
///
/// [`SaveThemeError::Serialize`] wrapping the underlying RON serialization error.
pub fn serialize_theme_def(def: &UuidThemeDef) -> Result<String, SaveThemeError> {
    ron::ser::to_string_pretty(def, ron::ser::PrettyConfig::default())
        .map_err(|err| SaveThemeError::Serialize(err.to_string()))
}

/// Validate a draft for SAVE — the C6 default-floor rule (the loader honors a default floor that
/// must resolve, so a saved theme's default floor MUST be one of its own terrain UUIDs).
///
/// Returns the minted key + the validated default floor on success. Pure (no IO), so the C7
/// tests exercise the exact validation the save path runs.
///
/// # Errors
///
/// - [`SaveThemeError::EmptyName`] — the display name slugifies to nothing.
/// - [`SaveThemeError::NoTerrain`] — the theme has no terrain selected.
/// - [`SaveThemeError::DefaultFloorNotInTerrain`] — no default floor is chosen, or the chosen
///   one is not in the theme's own terrain palette (C6).
pub fn validate_for_save(draft: &ThemeDraft) -> Result<(), SaveThemeError> {
    if draft.display_name().trim().is_empty() {
        return Err(SaveThemeError::EmptyName);
    }
    if draft.terrain().is_empty() {
        return Err(SaveThemeError::NoTerrain);
    }
    match draft.default_floor() {
        Some(floor) if draft.has_terrain(floor) => Ok(()),
        _ => Err(SaveThemeError::DefaultFloorNotInTerrain),
    }
}

/// Build + serialize + WRITE a theme def to `assets/terrain/<slug>/<slug>.terrain_theme.ron`
/// (GTW-475 C5), or return the typed [`SaveThemeError`] (never a panic).
///
/// Validates the draft (the C6 default-floor rule + a non-empty name + a non-empty palette),
/// slugifies the display name to the dir + file stem, projects the draft to a [`UuidThemeDef`]
/// keyed by `key`, serializes it, creates the themed directory if absent, and writes the file.
/// Returns the resolved [`PathBuf`] on success so the caller can log it; the C7 tests reuse the
/// pure halves.
///
/// # Errors
///
/// Any [`SaveThemeError`] from validation, serialization, or the file write.
#[cfg(debug_assertions)]
pub fn write_theme(draft: &ThemeDraft, key: ThemeUuid) -> Result<PathBuf, SaveThemeError> {
    validate_for_save(draft)?;
    let slug = slugify(draft.display_name());
    if slug.is_empty() {
        return Err(SaveThemeError::EmptyName);
    }
    let def = draft_to_theme_def(draft, key);
    let serialized = serialize_theme_def(&def)?;
    let path = theme_save_path(&slug);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| SaveThemeError::Write(err.to_string()))?;
    }
    std::fs::write(&path, serialized).map_err(|err| SaveThemeError::Write(err.to_string()))?;
    Ok(path)
}
