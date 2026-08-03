//! Sprite draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_content_families::sprites::{SpriteDef, SpriteDefsFamily, SpriteName};

use super::draft::SpriteDraft;

/// Build a sprite name and def from a draft.
#[must_use]
pub fn draft_to_sprite_def(draft: &SpriteDraft) -> (SpriteName, SpriteDef) {
    let name = SpriteName::new(draft.name().trim().to_owned());
    (name, draft.def().clone())
}

/// File name for a sprite asset under the sprite defs folder.
#[must_use]
pub fn sprite_file_name(name: &SpriteName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed sprite")
    } else {
        stem
    };
    format!("{stem}.{}", SpriteDefsFamily::EXTENSION)
}

/// Full path for a sprite asset under `root`.
#[must_use]
pub fn sprite_save_path_in(root: &Path, name: &SpriteName) -> PathBuf {
    root.join(SpriteDefsFamily::FOLDER)
        .join(sprite_file_name(name))
}

/// Write sprite RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_sprite_in(
    root: &Path,
    name: &SpriteName,
    def: &SpriteDef,
) -> Result<PathBuf, RonSaveError> {
    let path = sprite_save_path_in(root, name);
    write_ron_pretty(&path, def)?;
    Ok(path)
}

/// Write sprite RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_sprite(name: &SpriteName, def: &SpriteDef) -> Result<PathBuf, RonSaveError> {
    write_sprite_in(Path::new(WORKSPACE_ASSETS_ROOT), name, def)
}
