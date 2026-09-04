//! Canvas zoom factor resource.

use bevy::prelude::*;

const MIN_ZOOM: f32 = 0.25;

const MAX_ZOOM: f32 = 4.0;

/// Multiplicative zoom scale for the map canvas.
#[derive(Resource, Deref, Clone, Copy, PartialEq, Debug)]
pub struct CanvasZoom(f32);

impl CanvasZoom {
    /// 1:1 zoom.
    #[must_use]
    pub const fn identity() -> Self {
        Self(1.0)
    }

    /// Scale by `factor`, clamped to `[0.25, 4.0]`.
    #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self((self.0 * factor).clamp(MIN_ZOOM, MAX_ZOOM))
    }

    /// Reset to identity zoom.
    #[must_use]
    pub const fn reset() -> Self {
        Self::identity()
    }
}
