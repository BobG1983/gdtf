//! The THEME-mode form's **projection + serialization + write** (GTW-475): turn the in-progress
//! [`ThemeDraft`] into a real [`UuidThemeDef`], serialize it to a `.terrain_theme.ron`, and write
//! it to `assets/content/terrain/<slug>/<slug>.terrain_theme.ron` so the GTW-487 theme loader
//! (`resolve_theme_defs`) resolves it into the
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry).
//!
//! [`draft_to_theme_def`] + [`serialize_theme_def`] + [`theme_save_path_in`] are PURE (no IO) so a
//! test can round-trip them without touching the assets tree (the C7 round-trip). The filesystem
//! write lives in [`write_theme_in`] (root-parameterized core, debug-only — the GTW-555
//! `write_terrain_in` precedent, so tests aim it at a `tempfile::TempDir`) and its thin
//! production wrapper [`write_theme`] (the whole `theme_form` save trigger is gated, the
//! GTW-474 terrain-save precedent).

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gdtf_assets::serialize_ron_pretty;
#[cfg(debug_assertions)]
use gdtf_assets::{ContentFamily, FileStem, WORKSPACE_ASSETS_ROOT};
use gdtf_battle_sim::level::{ThemeDisplayName, ThemeUuid, UuidThemeDef};
#[cfg(debug_assertions)]
use gdtf_content_families::ThemeDefsFamily;

use super::types::{SaveThemeError, ThemeDraft};

// GTW-634 C1/C3: the assets root, the (mixed) per-theme terrain folder, and the compound
// extension are NOT re-spelled here — the root is the shared [`WORKSPACE_ASSETS_ROOT`]
// owner and the folder/extension are DERIVED from [`ThemeDefsFamily`]'s
// `FOLDER`/`EXTENSION` (the exact values the GTW-487 theme loader walks + dispatches on;
// the filename STEM is cosmetic, the `key` field is the UUID), so a saved theme lands
// where the loader reads BY CONSTRUCTION.

/// The `snake_case` slug for a theme, derived from its human display name — the TERRAIN form's
/// `theme_dir` sibling, since GTW-577 a thin delegation to the shared
/// [`gdtf_assets::sanitize_file_stem`] helper. Both the per-theme DIRECTORY and the file STEM
/// key on this slug (`"Industrial Hive"` → `industrial_hive`), matching the shipped per-theme
/// layout (`assets/content/terrain/underhive/underhive.terrain_theme.ron`).
///
/// Returns an empty [`FileStem`] for a name that slugifies to nothing (the caller treats it as
/// [`SaveThemeError::EmptyName`]).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn slugify(raw: &str) -> FileStem {
    gdtf_assets::sanitize_file_stem(raw)
}

/// The full on-disk PATH a saved theme def is written to under an arbitrary assets `root`:
/// `<root>/content/terrain/<slug>/<slug>.terrain_theme.ron` (GTW-475 C5) — the
/// root-parameterized core (the GTW-555 pattern), so a test resolves the REAL save location
/// against a `TempDir` root instead of the version-controlled `assets/` tree (GTW-662).
///
/// Pure (no IO) so a test can assert the resolved location without writing. `slug` is the
/// slugified display name (the same value names both the dir and the file stem).
#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn theme_save_path_in(root: &Path, slug: &str) -> PathBuf {
    root.join(ThemeDefsFamily::FOLDER)
        .join(slug)
        .join(format!("{slug}.{}", ThemeDefsFamily::EXTENSION))
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
/// schema the GTW-487 theme loader (`resolve_theme_defs`) deserializes (C3/C5). Delegates to
/// the shared [`serialize_ron_pretty`] helper (GTW-577 C2), so a saved def stays human-editable
/// like the shipped `assets/content/terrain/**/*.terrain_theme.ron`.
///
/// # Errors
///
/// [`SaveThemeError::Save`] wrapping the shared serializer's failure.
pub fn serialize_theme_def(def: &UuidThemeDef) -> Result<String, SaveThemeError> {
    serialize_ron_pretty(def).map_err(SaveThemeError::Save)
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

/// Build + serialize + WRITE a theme def to
/// `<assets_root>/content/terrain/<slug>/<slug>.terrain_theme.ron` (GTW-475 C5), or return the
/// typed [`SaveThemeError`] (never a panic).
///
/// This is the **root-parameterized core** (GTW-662 — the GTW-555 `write_terrain_in`
/// precedent): all path-building, serialization, and `fs` writes go through here.
/// `assets_root` is the on-disk parent of the `content/terrain/` subtree: production passes
/// [`WORKSPACE_ASSETS_ROOT`] (via [`write_theme`]); tests pass a unique `tempfile::TempDir`
/// root so no test ever writes into the version-controlled `assets/` tree.
///
/// Validates the draft (the C6 default-floor rule + a non-empty name + a non-empty palette),
/// slugifies the display name to the dir + file stem (the shared helper), projects the draft to
/// a [`UuidThemeDef`] keyed by `key`, and hands the serialize → mkdir → write chain to the
/// shared [`gdtf_assets::write_ron_pretty`] helper (GTW-577 C2). Returns the resolved
/// [`PathBuf`] on success so the caller can log it; the C7 tests reuse the pure halves.
///
/// # Errors
///
/// Any [`SaveThemeError`] from validation or the shared helper's serialization / file write.
#[cfg(debug_assertions)]
pub fn write_theme_in(
    assets_root: &Path,
    draft: &ThemeDraft,
    key: ThemeUuid,
) -> Result<PathBuf, SaveThemeError> {
    validate_for_save(draft)?;
    let slug = slugify(draft.display_name());
    if slug.is_empty() {
        return Err(SaveThemeError::EmptyName);
    }
    let def = draft_to_theme_def(draft, key);
    let path = theme_save_path_in(assets_root, &slug);
    gdtf_assets::write_ron_pretty(&path, &def)?;
    Ok(path)
}

/// Build + serialize + WRITE a theme def to `assets/content/terrain/<slug>/<slug>.terrain_theme.ron`
/// (GTW-475 C5), or return the typed [`SaveThemeError`] (never a panic).
///
/// Thin wrapper around [`write_theme_in`] that supplies the workspace `assets/` root
/// ([`WORKSPACE_ASSETS_ROOT`]) — identical paths for production callers (GTW-662 C3).
/// This is the function the egui Save button calls; the file lands exactly where the GTW-487
/// theme loader (`resolve_theme_defs`) reads from.
///
/// # Errors
///
/// Any [`SaveThemeError`] from validation or the shared helper's serialization / file write
/// (see [`write_theme_in`]).
#[cfg(debug_assertions)]
pub fn write_theme(draft: &ThemeDraft, key: ThemeUuid) -> Result<PathBuf, SaveThemeError> {
    write_theme_in(Path::new(WORKSPACE_ASSETS_ROOT), draft, key)
}
