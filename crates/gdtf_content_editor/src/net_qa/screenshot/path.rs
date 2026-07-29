//! The confined, per-capture-UNIQUE on-disk path for an editor QA screenshot (GTW-880).
//!
//! The same two properties the game's T7 path module guarantees
//! (`crates/gdtf_app/src/dev/net_qa/screenshot/path.rs`), applied to the editor's own
//! output directory:
//!
//! - **Confinement.** A [`ShotName`] arrives off the network and is UNTRUSTED: it may carry
//!   `../`, an absolute prefix, or nested directories. [`confine_shot_path`] reduces it to a
//!   single safe file component joined under [`EditorQaShotDir`], so a capture can NEVER
//!   write outside that one directory.
//! - **Uniqueness.** Every claimed capture gets its OWN path — [`next_capture_path`] folds a
//!   monotonic [`EditorShotSequence`] into the file name (`<stem>_<n>.png`), so a repeated
//!   name (or a leftover from a prior run) never shares a file the pump could mistake for
//!   THIS capture's output.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use gdtf_qa_protocol::ids::ShotName;
use gdtf_screenshot::CapturePath;

/// The directory every editor QA screenshot is confined to, relative to the run's working
/// directory (the workspace root for `cargo run`).
///
/// Named-newtype [`Resource`] over [`PathBuf`] (no-bare-types) with a [`Default`] of
/// `target/editor_qa_screenshots` — `target/` is writable in dev and excluded from release
/// artifacts, and the directory is DISTINCT from the game's `target/qa_screenshots` so a QA
/// session running both hosts at once can tell the two captures apart at a glance. Every
/// wire-supplied name is confined STRICTLY inside this directory. The inner is PRIVATE.
///
/// `pub` (re-exported from the crate root under the same `net_qa` gate) so the integration
/// suite can point a capture at a temp directory instead of the source tree.
#[derive(Resource, Clone, Debug, Deref)]
pub struct EditorQaShotDir(PathBuf);

impl EditorQaShotDir {
    /// Wrap an explicit confinement directory — the integration suite's temp-dir override
    /// (the plugin's own wiring uses the [`Default`]).
    #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self(dir)
    }
}

impl Default for EditorQaShotDir {
    /// The wiring default: `target/editor_qa_screenshots`.
    fn default() -> Self {
        Self(PathBuf::from("target/editor_qa_screenshots"))
    }
}

/// A monotonic per-capture counter that makes every capture's output path unique.
///
/// Private-inner newtype [`Resource`] over `u64` (no-bare-types): the pump
/// [`advance`](Self::advance)s it once per claimed capture and folds the value into the file
/// name (`<stem>_<n>.png`). Uniqueness means a repeated name — or a decodable PNG left at a
/// path by an earlier capture — never becomes a file a later capture's poll could mistake for
/// its own output.
#[derive(Resource, Clone, Copy, Debug, Default, Deref)]
pub(in crate::net_qa) struct EditorShotSequence(u64);

impl EditorShotSequence {
    /// Take the current value and advance the counter, so each claimed capture is handed a
    /// DISTINCT sequence number (and therefore a distinct output path).
    const fn advance(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

/// The file stem used when a request supplies no name (or a name that filters down to
/// nothing). A fixed label handed to a path, not a domain value.
const DEFAULT_STEM: &str = "editor_qa_shot";

/// Confine an untrusted wire [`ShotName`] to a `.png` path strictly inside `dir`.
///
/// A client name may carry `../`, an absolute prefix, or nested directories.
/// [`Path::file_name`] discards every leading directory / traversal segment, the remainder is
/// filtered down to `[A-Za-z0-9_-]`, and that SINGLE component is joined under `dir` — so the
/// result can never escape it. An empty / fully-stripped name falls back to [`DEFAULT_STEM`];
/// the lone `.png` extension is applied here (any client-supplied extension is dropped by the
/// filter). This proves CONFINEMENT only; [`next_capture_path`] adds the per-capture
/// uniqueness suffix on top.
fn confine_shot_path(dir: &EditorQaShotDir, name: Option<&ShotName>) -> CapturePath {
    let raw = name.map_or("", |shot| shot.as_str());
    // Keep only the final path component (drops any directory / `..` segment) ...
    let component = Path::new(raw)
        .file_name()
        .and_then(|last| last.to_str())
        .unwrap_or("");
    // ... drop a client-supplied `.png` so it is not mangled into the stem (re-applied
    // below), then filter to a safe single-component charset.
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

/// Fold an [`EditorShotSequence`] value into an already-confined path's file name, yielding
/// `<stem>_<n>.png` in the SAME directory.
///
/// `base` is the confined single-component path [`confine_shot_path`] produced, so replacing
/// only its file name (keeping its parent, appending the numeric suffix + `.png`) stays
/// strictly under [`EditorQaShotDir`] — the confinement guarantee is preserved.
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

/// The pump's path entry point: confine the untrusted `name` under `dir`, then make it UNIQUE
/// by folding in the next [`EditorShotSequence`] value — so no two claimed captures ever share
/// a file to clobber (a repeated name, or a leftover from a prior run).
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

    /// A traversing / absolute / nested wire name is reduced to ONE file component under the
    /// confinement directory — a capture can never write outside it.
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
        // An empty name still lands on the fallback stem, inside the same directory.
        let fallback = next_capture_path(&dir, None, &mut seq);
        assert_eq!(
            fallback,
            gdtf_screenshot::CapturePath::new(
                PathBuf::from("target/test_shots").join(format!("{DEFAULT_STEM}_0.png")),
            ),
        );
    }

    /// A name carrying shell metacharacters, spaces or separators keeps only
    /// `[A-Za-z0-9_-]` in its stem — the charset filter, asserted apart from the
    /// `file_name` reduction so each mechanism is falsifiable on its own.
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

    /// Two captures asking for the SAME name get DIFFERENT paths — the sequence suffix is
    /// what stops one capture's poll from reading another's bytes.
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
