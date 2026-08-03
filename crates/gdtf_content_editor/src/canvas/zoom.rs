use bevy::prelude::*;

const MIN_ZOOM: f32 = 0.25;

const MAX_ZOOM: f32 = 4.0;

#[derive(Resource, Deref, Clone, Copy, PartialEq, Debug)]
pub struct CanvasZoom(f32);

impl CanvasZoom {
        #[must_use]
    pub const fn identity() -> Self {
        Self(1.0)
    }

            #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self((self.0 * factor).clamp(MIN_ZOOM, MAX_ZOOM))
    }

        #[must_use]
    pub const fn reset() -> Self {
        Self::identity()
    }
}
