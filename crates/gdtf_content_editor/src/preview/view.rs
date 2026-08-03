use bevy::prelude::*;

use crate::canvas::CanvasZoom;

#[cfg(test)]
const MIN_PREVIEW_SCALE: f32 = 0.25;

#[cfg(test)]
const MAX_PREVIEW_SCALE: f32 = 4.0;

const ZOOM_STEP: f32 = 1.1;

#[derive(Resource, Deref, Clone, Copy, PartialEq, Debug)]
pub struct PreviewPan(Vec2);

impl PreviewPan {
        #[must_use]
    pub const fn origin() -> Self {
        Self(Vec2::ZERO)
    }

            #[must_use]
    pub const fn with_offset(offset: Vec2) -> Self {
        Self(offset)
    }

        #[must_use]
    pub const fn offset(self) -> Vec2 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct ZoomOutcome {
    /// The new absolute zoom factor (clamped to `[MIN_PREVIEW_SCALE, MAX_PREVIEW_SCALE]`).
    zoom: CanvasZoom,
        pan:  PreviewPan,
}

impl ZoomOutcome {
        #[must_use]
    pub(crate) const fn zoom(self) -> CanvasZoom {
        self.zoom
    }

        #[must_use]
    pub(crate) const fn pan(self) -> PreviewPan {
        self.pan
    }
}

#[must_use]
pub(crate) fn cursor_anchored_zoom(
    current_zoom: CanvasZoom,
    current_pan: PreviewPan,
    cursor_world: Vec2,
    notches: f32,
) -> ZoomOutcome {
    let old_scale = *current_zoom;
    let factor = ZOOM_STEP.powf(-notches);
    let new_zoom = current_zoom.scaled(factor);
    let new_scale = *new_zoom;
    // `pan' = w - (w - pan) * (s'/s)`. Guard the divide (old_scale is always >= MIN_PREVIEW_SCALE
    let ratio = if old_scale.abs() > f32::EPSILON {
        new_scale / old_scale
    } else {
        1.0
    };
    let new_offset = cursor_world - (cursor_world - current_pan.offset()) * ratio;
    ZoomOutcome {
        zoom: new_zoom,
        pan:  PreviewPan::with_offset(new_offset),
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;

    use super::{
        CanvasZoom, MAX_PREVIEW_SCALE, MIN_PREVIEW_SCALE, PreviewPan, cursor_anchored_zoom,
    };

            #[test]
    fn positive_notch_zooms_in_negative_zooms_out() {
        let start = CanvasZoom::identity();
        let zoomed_in = cursor_anchored_zoom(start, PreviewPan::origin(), Vec2::ZERO, 1.0).zoom();
        assert!(
            *zoomed_in < *start,
            "a positive wheel notch must DECREASE the ortho scale (zoom in): {} !< {}",
            *zoomed_in,
            *start,
        );
        let zoomed_out = cursor_anchored_zoom(start, PreviewPan::origin(), Vec2::ZERO, -1.0).zoom();
        assert!(
            *zoomed_out > *start,
            "a negative wheel notch must INCREASE the ortho scale (zoom out): {} !> {}",
            *zoomed_out,
            *start,
        );
    }

        /// zoom-in notches saturate at `MIN_PREVIEW_SCALE`, many zoom-out notches at
    /// `MAX_PREVIEW_SCALE` (the ticket fixes these bounds, so pinning THEM is allowed — C4.13).
    #[test]
    fn scale_clamps_to_the_system_constant_band() {
        let mut zoom = CanvasZoom::identity();
        for _ in 0..200 {
            zoom = cursor_anchored_zoom(zoom, PreviewPan::origin(), Vec2::ZERO, 1.0).zoom();
        }
        assert!(
            (*zoom - MIN_PREVIEW_SCALE).abs() < f32::EPSILON,
            "repeated zoom-in must saturate at MIN_PREVIEW_SCALE (0.25), got {}",
            *zoom,
        );
        let mut zoom = CanvasZoom::identity();
        for _ in 0..200 {
            zoom = cursor_anchored_zoom(zoom, PreviewPan::origin(), Vec2::ZERO, -1.0).zoom();
        }
        assert!(
            (*zoom - MAX_PREVIEW_SCALE).abs() < f32::EPSILON,
            "repeated zoom-out must saturate at MAX_PREVIEW_SCALE (4.0), got {}",
            *zoom,
        );
    }

                #[test]
    fn zoom_is_anchored_to_the_cursor() {
        let start = CanvasZoom::identity();
        let pan = PreviewPan::with_offset(Vec2::new(10.0, -4.0));
        let cursor = Vec2::new(7.0, 3.0);

        let before_offset = (cursor - pan.offset()) / *start;
        let outcome = cursor_anchored_zoom(start, pan, cursor, 1.0);
        let after_offset = (cursor - outcome.pan().offset()) / *outcome.zoom();

        assert!(
            (before_offset - after_offset).length() < 1.0e-4,
            "the cursor's screen offset must be invariant across the zoom (cursor-anchored): \
             {before_offset:?} != {after_offset:?}",
        );
    }

            #[test]
    fn origin_cursor_keeps_origin_pan() {
        let outcome = cursor_anchored_zoom(
            CanvasZoom::identity(),
            PreviewPan::origin(),
            Vec2::ZERO,
            2.0,
        );
        assert!(
            outcome.pan().offset().length() < f32::EPSILON,
            "zooming with the cursor at the origin and an origin pan must not drift the pan, got {:?}",
            outcome.pan().offset(),
        );
    }
}
