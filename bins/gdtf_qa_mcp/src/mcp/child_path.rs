//! [`ChildReportedPath`] → [`resolve_child_path`] → [`ChildFilePath`] — where the MCP host
//! reads a file the CHILD wrote (GTW-923; widened past screenshots in GTW-942).
//!
//! # The property this file exists to hold
//!
//! **The host never reads a child-written file path against its OWN current
//! directory.** A saved path in a reply is written by the child, under the child's own
//! relative default (`target/qa_screenshots/` for the game, `target/editor_qa_screenshots/`
//! for the editor), so it only names a real file when read relative to the directory the
//! child RAN IN. Reading it against the host's directory worked purely by coincidence —
//! whenever the two happened to be the same checkout — and failed the moment a launch named
//! a `working_dir` of its own, which is the whole point of `launch`'s `working_dir`
//! argument (GTW-875). A capture taken in a git worktree came back as
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

use gdtf_qa_protocol::command::ArtifactPath;

use crate::lifecycle::WorkingDir;

/// A path a CHILD reported, in the child's own words — the input side of the resolve.
///
/// [`ArtifactPath`] — a command reply's attachment — is what carries one. A borrowed newtype
/// over it, so [`resolve_child_path`] states in its signature what it accepts instead of
/// taking a bare `&str` that any string would satisfy.
///
/// The `From` impl below is the ONLY way to build one. That is the point: a
/// [`ChildFilePath`] turned back into text cannot be handed to [`resolve_child_path`],
/// which is the mistake GTW-923 fixed and this type keeps fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ChildReportedPath<'a>(&'a str);

impl<'a> From<&'a ArtifactPath> for ChildReportedPath<'a> {
    fn from(path: &'a ArtifactPath) -> Self {
        Self(path.as_str())
    }
}

impl Deref for ChildReportedPath<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// The **host-side filesystem path** a child-written file is read from — the child's
/// reported path resolved against the directory the child ran in.
///
/// Private-inner newtype over `PathBuf` (no-bare-types). Distinct from
/// `ArtifactPath` on purpose: that is what the CHILD reported, in
/// the child's own terms, and this one is what the HOST may hand to [`std::fs::read`].
/// Collapsing them is exactly the mistake GTW-923 fixes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ChildFilePath(PathBuf);

impl ChildFilePath {
    /// Build a host-side read path from its resolved location.
    #[must_use]
    const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Deref for ChildFilePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Resolve the path a child reported into the path the HOST reads.
///
/// Takes a [`ChildReportedPath`] because a command's attachment ([`ArtifactPath`]) is not a
/// host-side path, which is what this signature and its return type say.
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
pub(super) fn resolve_child_path(
    reported: ChildReportedPath<'_>,
    child_dir: Option<&WorkingDir>,
) -> ChildFilePath {
    let reported_path = Path::new(&*reported);
    if reported_path.is_absolute() {
        return ChildFilePath::new(reported_path.to_path_buf());
    }
    match child_dir {
        Some(dir) => ChildFilePath::new(dir.join(reported_path)),
        None => ChildFilePath::new(reported_path.to_path_buf()),
    }
}
