use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::weapon::{MeleeWeaponSpec, WeaponName};
use gdtf_content_families::MeleeWeaponsFamily;

use super::draft::MeleeWeaponDraft;

#[must_use]
pub fn draft_to_melee_weapon_spec(draft: &MeleeWeaponDraft) -> (WeaponName, MeleeWeaponSpec) {
    let name = WeaponName::new(draft.name().trim().to_owned());
    (name, draft.spec().clone())
}

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

#[must_use]
pub fn melee_weapon_save_path_in(root: &Path, name: &WeaponName) -> PathBuf {
    root.join(MeleeWeaponsFamily::FOLDER)
        .join(melee_weapon_file_name(name))
}

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

#[cfg(debug_assertions)]
pub fn write_melee_weapon(
    name: &WeaponName,
    spec: &MeleeWeaponSpec,
) -> Result<PathBuf, RonSaveError> {
    write_melee_weapon_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
