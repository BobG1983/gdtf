use std::path::{Path, PathBuf};

use bevy::prelude::*;
use gdtf_qa_protocol::ids::ShotName;
use gdtf_screenshot::CapturePath;

crate::support_item! {
    /// The directory QA screenshots are written to.
    #[derive(Resource, Clone, Debug, Deref)]
    struct QaShotDir(PathBuf);
}

impl QaShotDir {
    crate::support_item! {
        /// Point screenshot writes at this directory.
        #[cfg(any(test, feature = "headless_test"))]
        const fn new(dir: PathBuf) -> Self {
            Self(dir)
        }
    }
}

impl Default for QaShotDir {
    fn default() -> Self {
        Self(PathBuf::from("target/qa_screenshots"))
    }
}

#[derive(Resource, Clone, Copy, Debug, Default, Deref)]
pub(in crate::dev::net_qa) struct ShotSequence(u64);

impl ShotSequence {
    const fn advance(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

const DEFAULT_STEM: &str = "qa_shot";

pub(in crate::dev::net_qa) fn confine_shot_path(
    dir: &QaShotDir,
    name: Option<&ShotName>,
) -> CapturePath {
    let raw = name.map_or("", |shot| shot.as_str());
    let component = Path::new(raw)
        .file_name()
        .and_then(|last| last.to_str())
        .unwrap_or("");
    let stem_src = component.strip_suffix(".png").unwrap_or(component);
    let cleaned: String = stem_src
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect();
    let stem = if cleaned.is_empty() {
        DEFAULT_STEM
    } else {
        cleaned.as_str()
    };
    CapturePath::new(dir.join(format!("{stem}.png")))
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

pub(in crate::dev::net_qa) fn next_capture_path(
    dir: &QaShotDir,
    name: Option<&ShotName>,
    seq: &mut ShotSequence,
) -> CapturePath {
    let base = confine_shot_path(dir, name);
    sequenced_path(&base, seq.advance())
}
