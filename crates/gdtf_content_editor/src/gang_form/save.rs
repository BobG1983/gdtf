//! Gang draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use gdtf_assets::{ContentFamily, sanitize_file_stem};
#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, WORKSPACE_ASSETS_ROOT, write_ron_pretty};
use gdtf_battle_sim::ganger::{GangName, GangRoster};
use gdtf_content_families::GangsFamily;

use super::draft::GangDraft;

/// Build a gang name and roster from a draft.
#[must_use]
pub fn draft_to_roster(draft: &GangDraft) -> (GangName, GangRoster) {
    let name = GangName::new(draft.name().trim().to_owned());
    let roster = GangRoster::new(draft.members().iter().cloned());
    (name, roster)
}

/// File name for a gang asset under the gangs family folder.
#[must_use]
pub fn gang_file_name(name: &GangName) -> String {
    let stem = sanitize_file_stem(name.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed gang")
    } else {
        stem
    };
    format!("{stem}.{}", GangsFamily::EXTENSION)
}

/// Full path for a gang asset under `root`.
#[must_use]
pub fn gang_save_path_in(root: &Path, name: &GangName) -> PathBuf {
    root.join(GangsFamily::FOLDER).join(gang_file_name(name))
}

/// Write gang RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_gang_in(
    root: &Path,
    name: &GangName,
    roster: &GangRoster,
) -> Result<PathBuf, RonSaveError> {
    let path = gang_save_path_in(root, name);
    write_ron_pretty(&path, roster)?;
    Ok(path)
}

/// Write gang RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_gang(name: &GangName, roster: &GangRoster) -> Result<PathBuf, RonSaveError> {
    write_gang_in(Path::new(WORKSPACE_ASSETS_ROOT), name, roster)
}
