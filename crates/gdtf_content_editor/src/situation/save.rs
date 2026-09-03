//! Writing the authored situation back to an explicit assets root.

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

#[cfg(debug_assertions)]
use gdtf_assets::{RonSaveError, write_ron_pretty};
#[cfg(debug_assertions)]
use gdtf_battle_sim::situation::Situation;
#[cfg(debug_assertions)]
use gdtf_content_families::situation::SITUATION_RON_PATH;

#[cfg(test)]
mod test;

/// Write the whole situation under `root`.
///
/// # Errors
///
/// Returns [`RonSaveError`] if the path cannot be written.
#[cfg(debug_assertions)]
pub fn write_situation_in(root: &Path, situation: &Situation) -> Result<PathBuf, RonSaveError> {
    let path = root.join(SITUATION_RON_PATH);
    write_ron_pretty(&path, situation)?;
    Ok(path)
}
