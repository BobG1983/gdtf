//! Weapon draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, workspace_assets_root, write_ron_pretty};
use gdtf_battle_sim::weapon::{WeaponName, WeaponSpec};
use gdtf_content_families::WeaponsFamily;

use super::draft::WeaponDraft;

/// Build a weapon name and spec from a draft.
#[must_use]
pub fn draft_to_weapon_spec(draft: &WeaponDraft) -> (WeaponName, WeaponSpec) {
    let name = WeaponName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

/// File name for a weapon asset under the weapons family folder.
#[must_use]
pub fn weapon_file_name(name: &WeaponName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed weapon")
    } else {
        stem
    };
    format!("{stem}.{}", WeaponsFamily::EXTENSION)
}

/// Full path for a weapon asset under `root`.
#[must_use]
pub fn weapon_save_path_in(root: &Path, name: &WeaponName) -> PathBuf {
    root.join(WeaponsFamily::FOLDER)
        .join(weapon_file_name(name))
}

/// Write weapon RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_weapon_in(
    root: &Path,
    name: &WeaponName,
    spec: &WeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = weapon_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

/// Write weapon RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the workspace root search fails or the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_weapon(name: &WeaponName, spec: &WeaponSpec) -> Result<PathBuf, RonSaveError> {
    let Some(root) = workspace_assets_root() else {
        return Err(RonSaveError::NoWorkspaceRoot);
    };
    write_weapon_in(&root, name, spec)
}
