//! What one in-flight capture writes, and the image it reads.

use bevy::prelude::*;

use super::target::CaptureImage;
use crate::path::CapturePath;

/// Destination of one in-flight capture, and the image its readback reads.
#[derive(Component, Debug, Clone)]
pub(crate) struct CaptureShot {
    path:  CapturePath,
    image: CaptureImage,
}

impl CaptureShot {
    /// Record where this capture writes and what it reads.
    #[must_use]
    pub(crate) const fn new(path: CapturePath, image: CaptureImage) -> Self {
        Self { path, image }
    }

    /// Where the PNG is written.
    #[must_use]
    pub(super) const fn path(&self) -> &CapturePath {
        &self.path
    }

    /// The image the readback reads.
    #[must_use]
    pub(super) const fn image(&self) -> &CaptureImage {
        &self.image
    }
}
