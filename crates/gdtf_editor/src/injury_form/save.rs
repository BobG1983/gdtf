//! Injury and weighting draft conversion and RON save helpers.

use std::path::{Path, PathBuf};

use cobalt_ron_assets::sanitize_file_stem;
#[cfg(feature = "mcp")]
use cobalt_ron_assets::{RonSaveError, workspace_assets_root, write_ron_pretty};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryDef, InjuryName, InjuryWeighting},
};
use gdtf_content_families::injuries::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, WEIGHTING_SUBFOLDER,
    category_dir, weighting_context_infix,
};

use super::{draft::InjuryDraft, weighting::WeightingDraft};

/// Build an injury name and def from a draft.
#[must_use]
pub fn draft_to_def(draft: &InjuryDraft) -> (InjuryName, InjuryDef) {
    let key = InjuryName::new(draft.key().trim().to_owned());
    (key, draft.def().clone())
}

/// Build a weighting from a draft.
#[must_use]
pub fn draft_to_weighting(draft: &WeightingDraft) -> InjuryWeighting {
    draft.weighting().clone()
}

/// File name for an injury def.
#[must_use]
pub fn injury_file_name(key: &InjuryName) -> String {
    let stem = sanitize_file_stem(key.as_str());
    let stem = if stem.is_empty() {
        sanitize_file_stem("unnamed injury")
    } else {
        stem
    };
    format!("{stem}.{INJURY_DEF_EXTENSION}")
}

/// Full path for an injury def under `root`.
#[must_use]
pub fn injury_save_path_in(root: &Path, category: InjuryCategory, key: &InjuryName) -> PathBuf {
    root.join(INJURIES_FOLDER)
        .join(category_dir(category))
        .join(injury_file_name(key))
}

/// File name for a weighting asset.
#[must_use]
pub fn weighting_file_name(category: InjuryCategory, context: DamageContext) -> String {
    match weighting_context_infix(context) {
        Some(infix) => format!(
            "{}.{infix}.{INJURY_WEIGHTING_EXTENSION}",
            category_dir(category)
        ),
        None => format!("{}.{INJURY_WEIGHTING_EXTENSION}", category_dir(category)),
    }
}

/// Full path for a weighting asset under `root`.
#[must_use]
pub fn weighting_save_path_in(
    root: &Path,
    category: InjuryCategory,
    context: DamageContext,
) -> PathBuf {
    root.join(INJURIES_FOLDER)
        .join(WEIGHTING_SUBFOLDER)
        .join(weighting_file_name(category, context))
}

/// Write injury RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(feature = "mcp")]
pub fn write_injury_in(
    root: &Path,
    key: &InjuryName,
    def: &InjuryDef,
) -> Result<PathBuf, RonSaveError> {
    let path = injury_save_path_in(root, def.category, key);
    write_ron_pretty(&path, def)?;
    Ok(path)
}

/// Write injury RON under the workspace assets root.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the workspace root search fails or the path cannot be written.
#[cfg(feature = "mcp")]
pub fn write_injury(key: &InjuryName, def: &InjuryDef) -> Result<PathBuf, RonSaveError> {
    let Some(root) = workspace_assets_root() else {
        return Err(RonSaveError::NoWorkspaceRoot);
    };
    write_injury_in(&root, key, def)
}

/// Write weighting RON under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(feature = "mcp")]
pub fn write_weighting_in(
    root: &Path,
    weighting: &InjuryWeighting,
) -> Result<PathBuf, RonSaveError> {
    let path = weighting_save_path_in(root, weighting.category, weighting.context);
    write_ron_pretty(&path, weighting)?;
    Ok(path)
}
