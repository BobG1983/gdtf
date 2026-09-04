use core::ops::Deref;
use std::path::{Path, PathBuf};

use cobalt_mcp_protocol::command::ArtifactPath;

use crate::lifecycle::WorkingDir;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ChildFilePath(PathBuf);

impl ChildFilePath {
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
