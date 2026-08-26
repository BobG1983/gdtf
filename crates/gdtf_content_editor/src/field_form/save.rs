//! Field draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, workspace_assets_root, write_ron_pretty};
use gdtf_battle_sim::effects::fields::{FieldDef, FieldKey};
use gdtf_content_families::FieldsFamily;

use super::draft::FieldDraft;

/// Build a field key and def from a draft.
#[must_use]
pub fn draft_to_field(draft: &FieldDraft) -> (FieldKey, FieldDef) {
    let key = FieldKey::new(draft.key().trim().to_owned());
    (key, draft.def().clone())
}

/// File name for a field asset under the fields family folder.
#[must_use]
pub fn field_file_name(key: &FieldKey) -> String {
    let stem = sanitize_file_stem(key.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed field")
    } else {
        stem
    };
    format!("{stem}.{}", FieldsFamily::EXTENSION)
}

/// Full path for a field asset under `root`.
#[must_use]
pub fn field_save_path_in(root: &Path, key: &FieldKey) -> PathBuf {
    root.join(FieldsFamily::FOLDER).join(field_file_name(key))
}

/// Write field RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_field_in(
    root: &Path,
    key: &FieldKey,
    def: &FieldDef,
) -> Result<PathBuf, RonSaveError> {
    let path = field_save_path_in(root, key);
    write_ron_pretty(&path, def)?;
    Ok(path)
}

/// Write field RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the workspace root search fails or the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_field(key: &FieldKey, def: &FieldDef) -> Result<PathBuf, RonSaveError> {
    let Some(root) = workspace_assets_root() else {
        return Err(RonSaveError::NoWorkspaceRoot);
    };
    write_field_in(&root, key, def)
}
