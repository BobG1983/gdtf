//! Shot directory and the confined, unique path each capture writes to.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use super::stem::ShotStem;
use crate::path::CapturePath;

/// Stem used when a caller names none.
pub(super) const DEFAULT_STEM: &str = "qa_shot";

/// Directory captures land in when nothing names another.
const DEFAULT_SHOT_DIR: &str = "qa_screenshots";

/// Name of one directory under the workspace `target/`.
#[derive(Clone, Debug, Deref)]
pub struct ShotDirName(String);

impl ShotDirName {
    /// Wrap a directory name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Directory every stem-named capture is written into.
#[derive(Resource, Clone, Debug, Deref)]
pub struct ShotDir(PathBuf);

impl ShotDir {
    /// Point stem-named captures at this directory.
    #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self(dir)
    }

    /// Point stem-named captures at a named directory under the workspace `target/`.
    #[must_use]
    pub fn under_workspace_target(name: &ShotDirName) -> Self {
        Self(workspace_target().join(&**name))
    }
}

impl Default for ShotDir {
    fn default() -> Self {
        Self::under_workspace_target(&ShotDirName::new(DEFAULT_SHOT_DIR))
    }
}

// A temp directory when the search fails, so a miss never writes inside the repo.
fn workspace_target() -> PathBuf {
    gdtf_assets::workspace_root()
        .unwrap_or_else(std::env::temp_dir)
        .join("target")
}

/// Counter that keeps repeated stems on distinct files.
#[derive(Resource, Clone, Copy, Debug, Default, Deref)]
pub(super) struct ShotSequence(u64);

impl ShotSequence {
    const fn advance(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

// Keeps only the final path component, minus any `.png`, minus anything outside
// `[A-Za-z0-9_-]`, so a wire-supplied name cannot leave the shot directory.
fn confine_shot_path(dir: &ShotDir, stem: Option<&ShotStem>) -> CapturePath {
    let raw = stem.map_or("", |requested| &**requested);
    let component = Path::new(raw)
        .file_name()
        .and_then(|last| last.to_str())
        .unwrap_or("");
    let stem_src = component.strip_suffix(".png").unwrap_or(component);
    let cleaned: String = stem_src
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect();
    let confined = if cleaned.is_empty() {
        DEFAULT_STEM
    } else {
        cleaned.as_str()
    };
    CapturePath::new(dir.join(format!("{confined}.png")))
}

fn sequenced_path(base: &CapturePath, seq: ShotSequence) -> CapturePath {
    let stem = base
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(DEFAULT_STEM);
    let file = format!("{stem}_{}.png", *seq);
    let path = base
        .parent()
        .map_or_else(|| PathBuf::from(&file), |dir| dir.join(&file));
    CapturePath::new(path)
}

pub(super) fn next_capture_path(
    dir: &ShotDir,
    stem: Option<&ShotStem>,
    seq: &mut ShotSequence,
) -> CapturePath {
    let base = confine_shot_path(dir, stem);
    sequenced_path(&base, seq.advance())
}
