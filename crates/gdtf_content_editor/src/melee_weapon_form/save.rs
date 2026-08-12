//! Melee weapon draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, workspace_assets_root, write_ron_pretty};
use gdtf_battle_sim::weapon::{MeleeWeaponSpec, WeaponName};
use gdtf_content_families::MeleeWeaponsFamily;

use super::draft::MeleeWeaponDraft;

/// Build a melee weapon name and spec from a draft.
#[must_use]
pub fn draft_to_melee_weapon_spec(draft: &MeleeWeaponDraft) -> (WeaponName, MeleeWeaponSpec) {
    let name = WeaponName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

/// File name for a melee weapon asset.
#[must_use]
pub fn melee_weapon_file_name(name: &WeaponName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed melee weapon")
    } else {
        stem
    };
    format!("{stem}.{}", MeleeWeaponsFamily::EXTENSION)
}

/// Full path for a melee weapon asset under `root`.
#[must_use]
pub fn melee_weapon_save_path_in(root: &Path, name: &WeaponName) -> PathBuf {
    root.join(MeleeWeaponsFamily::FOLDER)
        .join(melee_weapon_file_name(name))
}

/// Write melee weapon RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_melee_weapon_in(
    root: &Path,
    name: &WeaponName,
    spec: &MeleeWeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = melee_weapon_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

/// Write melee weapon RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the workspace root search fails or the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_melee_weapon(
    name: &WeaponName,
    spec: &MeleeWeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    let Some(root) = workspace_assets_root() else {
        return Err(RonSaveError::NoWorkspaceRoot);
    };
    write_melee_weapon_in(&root, name, spec)
}
