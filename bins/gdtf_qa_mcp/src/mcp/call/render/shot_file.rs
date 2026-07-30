//! [`ShotFilePath`] + [`resolve_shot_path`] — where the MCP host reads a capture the CHILD
//! wrote (GTW-923).
//!
//! # The property this file exists to hold
//!
//! **The host never reads a child-written screenshot path against its OWN current
//! directory.** A saved path in a reply is written by the child, under the child's own
//! relative default (`target/qa_screenshots/` for the game, `target/editor_qa_screenshots/`
//! for the editor), so it only names a real file when read relative to the directory the
//! child RAN IN. Reading it against the host's directory worked purely by coincidence —
//! whenever the two happened to be the same checkout — and failed the moment a launch named
//! a `working_dir` of its own, which is the whole point of `launch_editor working_dir:` /
//! `launch_game working_dir:` (GTW-875). A capture taken in a git worktree came back as
//! "screenshot saved to … but could not be read: No such file or directory" while the PNG
//! sat on disk, correct and complete.
//!
//! The fix is host-side: the launch recipe already carries the child's
//! [`WorkingDir`], the manager keeps the recipe of the child it is running, so the render
//! path asks for it and joins. No protocol change, no version bump, no child-side change —
//! and one rule covers BOTH children, because neither the join nor the reply knows which
//! host answered.

use core::ops::Deref;
use std::path::{Path, PathBuf};

use gdtf_qa_protocol::envelope::ScreenshotPathNet;

use crate::lifecycle::WorkingDir;

/// The **host-side filesystem path** a saved capture is read from — the child's reported
/// path resolved against the directory the child ran in.
///
/// Private-inner newtype over `PathBuf` (no-bare-types). Distinct from
/// [`ScreenshotPathNet`] on purpose: that one is what the CHILD reported, in the child's own
/// terms, and this one is what the HOST may hand to [`std::fs::read`]. Collapsing the two is
/// exactly the mistake GTW-923 fixes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ShotFilePath(PathBuf);

impl ShotFilePath {
    /// Build a host-side read path from its resolved location.
    #[must_use]
    pub(super) const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Deref for ShotFilePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Resolve the path a child reported into the path the HOST reads.
///
/// An already-absolute reported path is taken as it stands — the child named a file
/// system-wide and no directory can improve on that. A RELATIVE reported path is joined onto
/// `child_dir`, the directory the child was launched in, because that is the only directory
/// it was ever meaningful against.
///
/// `child_dir` is `None` only when the host cannot name the child's directory at all (no
/// child running, and the host's own current directory unreadable); the reported path is then
/// used unchanged, which is the most the host can honestly do. It is never a fabricated
/// success: an unresolvable or missing file still fails the read, and the caller reports it.
#[must_use]
pub(super) fn resolve_shot_path(
    reported: &ScreenshotPathNet,
    child_dir: Option<&WorkingDir>,
) -> ShotFilePath {
    let reported_path = Path::new(reported.as_str());
    if reported_path.is_absolute() {
        return ShotFilePath::new(reported_path.to_path_buf());
    }
    match child_dir {
        Some(dir) => ShotFilePath::new(dir.join(reported_path)),
        None => ShotFilePath::new(reported_path.to_path_buf()),
    }
}
