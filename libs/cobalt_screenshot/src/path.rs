//! Destination path for a screenshot capture.

use std::path::PathBuf;

use bevy::prelude::*;

/// Destination path for a screenshot PNG.
#[derive(Clone, Debug, PartialEq, Eq, Deref)]
pub struct CapturePath(PathBuf);

impl CapturePath {
    /// Wrap an owned path.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}
