//! Theme draft validation and RON save helpers.

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gdtf_assets::serialize_ron_pretty;
#[cfg(debug_assertions)]
use gdtf_assets::{ContentFamily, FileStem, WORKSPACE_ASSETS_ROOT};
use gdtf_battle_sim::level::{ThemeDisplayName, ThemeUuid, UuidThemeDef};
#[cfg(debug_assertions)]
use gdtf_content_families::ThemeDefsFamily;

use super::types::{SaveThemeError, ThemeDraft};

#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn slugify(raw: &str) -> FileStem {
    gdtf_assets::sanitize_file_stem(raw)
}

#[cfg(debug_assertions)]
#[must_use]
pub(crate) fn theme_save_path_in(root: &Path, slug: &str) -> PathBuf {
    root.join(ThemeDefsFamily::FOLDER)
        .join(slug)
        .join(format!("{slug}.{}", ThemeDefsFamily::EXTENSION))
}

/// Build a theme def from a draft and key.
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

/// Serialize a theme def to pretty RON.
///
/// # Errors
///
/// Returns [`SaveThemeError::Save`] if serialization fails.
pub fn serialize_theme_def(def: &UuidThemeDef) -> Result<String, SaveThemeError> {
    serialize_ron_pretty(def).map_err(SaveThemeError::Save)
}

/// Validate a draft before save.
///
/// # Errors
///
/// Returns [`SaveThemeError`] when the name is empty, terrain is empty, or the
/// default floor is not in the terrain list.
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

/// Write theme RON under `assets_root`.
///
/// # Errors
///
/// Returns [`SaveThemeError`] on validation or write failure.
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

/// Write theme RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`SaveThemeError`] on validation or write failure.
#[cfg(debug_assertions)]
pub fn write_theme(draft: &ThemeDraft, key: ThemeUuid) -> Result<PathBuf, SaveThemeError> {
    write_theme_in(Path::new(WORKSPACE_ASSETS_ROOT), draft, key)
}
