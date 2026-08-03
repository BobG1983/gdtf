use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::armor::{ArmorName, ArmorSpec};
use gdtf_content_families::ArmorFamily;

use super::draft::ArmorDraft;

#[must_use]
pub fn draft_to_spec(draft: &ArmorDraft) -> (ArmorName, ArmorSpec) {
    let name = ArmorName::new(draft.name().trim().to_owned());
    (name, *draft.spec())
}

#[must_use]
pub fn armor_file_name(name: &ArmorName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed armor")
    } else {
        stem
    };
    format!("{stem}.{}", ArmorFamily::EXTENSION)
}

#[must_use]
pub fn armor_save_path_in(root: &Path, name: &ArmorName) -> PathBuf {
    root.join(ArmorFamily::FOLDER).join(armor_file_name(name))
}

#[cfg(debug_assertions)]
pub fn write_armor_in(
    root: &Path,
    name: &ArmorName,
    spec: &ArmorSpec,
) -> Result<PathBuf, RonSaveError> {
    let path = armor_save_path_in(root, name);
    write_ron_pretty(&path, spec)?;
    Ok(path)
}

#[cfg(debug_assertions)]
pub fn write_armor(name: &ArmorName, spec: &ArmorSpec) -> Result<PathBuf, RonSaveError> {
    write_armor_in(Path::new(WORKSPACE_ASSETS_ROOT), name, spec)
}
