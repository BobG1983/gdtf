use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::weapon::{WeaponName, WeaponSpec};
use gdtf_content_families::WeaponsFamily;

use super::draft::WeaponDraft;

#[must_use]
pub fn draft_to_weapon_spec(draft: &WeaponDraft) -> (WeaponName, WeaponSpec) {
    let name = WeaponName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

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

#[must_use]
pub fn weapon_save_path_in(root: &Path, name: &WeaponName) -> PathBuf {
    root.join(WeaponsFamily::FOLDER)
        .join(weapon_file_name(name))
}

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

#[cfg(debug_assertions)]
pub fn write_weapon(name: &WeaponName, spec: &WeaponSpec) -> Result<PathBuf, RonSaveError> {
    write_weapon_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
