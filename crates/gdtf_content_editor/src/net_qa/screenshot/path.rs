use std::path::{Path, PathBuf};

use bevy::prelude::*;
use gdtf_qa_protocol::ids::ShotName;
use gdtf_screenshot::CapturePath;

#[derive(Resource, Clone, Debug, Deref)]
pub struct EditorQaShotDir(PathBuf);

impl EditorQaShotDir {
            #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self(dir)
    }
}

impl Default for EditorQaShotDir {
        fn default() -> Self {
        Self(PathBuf::from("target/editor_qa_screenshots"))
    }
}

#[derive(Resource, Clone, Copy, Debug, Default, Deref)]
pub(in crate::net_qa) struct EditorShotSequence(u64);

impl EditorShotSequence {
            const fn advance(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

const DEFAULT_STEM: &str = "editor_qa_shot";

fn confine_shot_path(dir: &EditorQaShotDir, name: Option<&ShotName>) -> CapturePath {
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

fn sequenced_path(base: &CapturePath, seq: EditorShotSequence) -> CapturePath {
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

pub(in crate::net_qa) fn next_capture_path(
    dir: &EditorQaShotDir,
    name: Option<&ShotName>,
    seq: &mut EditorShotSequence,
) -> CapturePath {
    let base = confine_shot_path(dir, name);
    sequenced_path(&base, seq.advance())
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use gdtf_qa_protocol::ids::ShotName;

    use super::{DEFAULT_STEM, EditorQaShotDir, EditorShotSequence, next_capture_path};

            #[test]
    fn a_traversing_name_is_confined_to_the_shot_directory() {
        let dir = EditorQaShotDir::new(PathBuf::from("target/test_shots"));
        let mut seq = EditorShotSequence::default();
        for hostile in ["../../etc/passwd", "/etc/passwd", "nested/dir/shot.png"] {
            let path = next_capture_path(
                &dir,
                Some(&ShotName::new(hostile.to_owned())),
                &mut EditorShotSequence::default(),
            );
            assert_eq!(
                path.parent(),
                Some(PathBuf::from("target/test_shots").as_path()),
                "{hostile:?} escaped the confinement directory: {}",
                path.display(),
            );
        }
        let fallback = next_capture_path(&dir, None, &mut seq);
        assert_eq!(
            fallback,
            gdtf_screenshot::CapturePath::new(
                PathBuf::from("target/test_shots").join(format!("{DEFAULT_STEM}_0.png")),
            ),
        );
    }

                #[test]
    fn a_hostile_charset_is_filtered_out_of_the_stem() {
        let dir = EditorQaShotDir::new(PathBuf::from("target/test_shots"));
        let hostile = ShotName::new("sh ell;$(rm -rf *)&|>'\"`.png".to_owned());
        let path = next_capture_path(&dir, Some(&hostile), &mut EditorShotSequence::default());
        let Some(file) = path.file_name().and_then(|name| name.to_str()) else {
            unreachable!("a confined path always has a file name");
        };
        assert_eq!(file, "shellrm-rf_0.png", "filtered file name: {file}");
    }

            #[test]
    fn a_repeated_name_still_yields_a_unique_path() {
        let dir = EditorQaShotDir::new(PathBuf::from("target/test_shots"));
        let mut seq = EditorShotSequence::default();
        let name = ShotName::new("shell".to_owned());
        let first = next_capture_path(&dir, Some(&name), &mut seq);
        let second = next_capture_path(&dir, Some(&name), &mut seq);
        assert_ne!(first, second, "a repeated name must not reuse a path");
    }
}
