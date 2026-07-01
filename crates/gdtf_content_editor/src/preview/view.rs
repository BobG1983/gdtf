//! The prefab **preview-viewport view state** (GTW-515 C4.7 / C4.8) — the owned zoom + pan
//! target the offscreen preview camera is driven to, plus the PURE cursor-anchored-zoom math the
//! wheel handler folds input through.
//!
//! ## Why an OWNED target, applied set-to-target (bevy-traps #8 fact (b))
//!
//! `bevy_egui` 0.41 runs the primary context in MULTI-PASS mode, so the egui UI closure can run up
//! to TWICE per frame (egui re-runs it to settle first-frame widget sizing). Any mutation done
//! INSIDE that closure that ACCUMULATES (`scale *= …`, `translation += …`) applies twice on a
//! discarded-then-rerun frame — double-speed zoom / pan. The fix (the research reference's
//! recommended split): the egui-pass handler folds input into these OWNED target resources by
//! computing an ABSOLUTE target (`target = current + delta`) and SETTING it — idempotent across
//! the two passes since egui reports the same `drag_delta` / `smooth_scroll_delta` on both — and a
//! SEPARATE once-per-frame system ([`apply_preview_view`](super::target::apply_preview_view))
//! writes the camera `Projection` / `Transform` from the target with a set-to-target assignment.
//!
//! ## The two owned targets
//!
//! - The zoom target is the KEPT [`CanvasZoom`](crate::canvas::CanvasZoom) factor — already a
//!   named newtype clamped to `[0.25, 4.0]` (GTW-500 C3), reused verbatim as the preview camera's
//!   [`OrthographicProjection::scale`]. Larger scale = zoomed OUT, so a wheel-UP (positive scroll)
//!   maps to a SMALLER scale (zoom in).
//! - [`PreviewPan`] — the owned pan OFFSET (a world-space translation) the preview camera's
//!   [`Transform`] centres on. Right-drag folds a screen-space `drag_delta` into it.

use bevy::prelude::*;

use crate::canvas::CanvasZoom;

/// The minimum preview-camera zoom scale — the same floor the [`CanvasZoom`] factor clamps to
/// (the ticket's fixed system constant `0.25`). Referenced by the C4.13 clamp test to assert the
/// saturation bound (the live clamp lives in [`CanvasZoom::scaled`], whose bounds equal these).
#[cfg(test)]
const MIN_PREVIEW_SCALE: f32 = 0.25;

/// The maximum preview-camera zoom scale — the same ceiling the [`CanvasZoom`] factor clamps to
/// (the ticket's fixed system constant `4.0`). Referenced by the C4.13 clamp test.
#[cfg(test)]
const MAX_PREVIEW_SCALE: f32 = 4.0;

/// The per-notch zoom step the wheel applies — each wheel notch multiplies the scale by this
/// factor (a MULTIPLY, so each notch is a constant proportional step). A framework tuning const
/// for the wheel feel (the no-bare-types clause-4 plumbing carve-out), not a domain value.
const ZOOM_STEP: f32 = 1.1;

/// The preview camera's **pan offset** (GTW-515 C4.8) — the world-space translation the offscreen
/// preview camera centres on, folded from right-drag.
///
/// A named newtype over the framework [`Vec2`] (no-bare-types rule 1: a viewport pan is a domain
/// value). PRIVATE inner, derived [`Deref`]; the offset is only ever SET absolutely (the
/// set-to-target contract — never `+=`), so it is safe to fold input into it inside the egui
/// multipass closure. A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed
/// `OnExit(Editing)` — bevy-traps #1), seeded to the origin.
#[derive(Resource, Deref, Clone, Copy, PartialEq, Debug)]
pub struct PreviewPan(Vec2);

impl PreviewPan {
    /// The un-panned offset — the preview centred on the world origin (the editor's open state).
    #[must_use]
    pub const fn origin() -> Self {
        Self(Vec2::ZERO)
    }

    /// This pan offset with `offset` SET as the new absolute value (the set-to-target contract —
    /// never accumulate; the caller computes `current + delta` and passes the absolute result).
    #[must_use]
    pub const fn with_offset(offset: Vec2) -> Self {
        Self(offset)
    }

    /// The world-space offset the preview camera's [`Transform`] centres on.
    #[must_use]
    pub const fn offset(self) -> Vec2 {
        self.0
    }
}

/// A wheel-zoom outcome (GTW-515 C4.7) — the ABSOLUTE zoom + pan the wheel handler SETS its owned
/// targets to after a cursor-anchored zoom. A named struct (not a bare `(CanvasZoom, PreviewPan)`
/// tuple) so the two absolute targets read self-describing at the call site.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct ZoomOutcome {
    /// The new absolute zoom factor (clamped to `[MIN_PREVIEW_SCALE, MAX_PREVIEW_SCALE]`).
    zoom: CanvasZoom,
    /// The new absolute pan offset (so the world point under the cursor stays fixed).
    pan:  PreviewPan,
}

impl ZoomOutcome {
    /// The new absolute zoom factor.
    #[must_use]
    pub(crate) const fn zoom(self) -> CanvasZoom {
        self.zoom
    }

    /// The new absolute pan offset.
    #[must_use]
    pub(crate) const fn pan(self) -> PreviewPan {
        self.pan
    }
}

/// The CURSOR-ANCHORED wheel-zoom math (GTW-515 C4.7) — pure, so the unit test exercises the real
/// path (C4.13).
///
/// Given the CURRENT zoom + pan, the cursor's position in WORLD space (its offscreen-camera world
/// coordinate before the zoom), and a signed wheel `notches` count, returns the new absolute
/// zoom-and-pan such that the world point under the cursor stays FIXED on screen (zoom toward the
/// cursor).
///
/// The new scale is `current * ZOOM_STEP^(-notches)` clamped to `[MIN, MAX]` — a positive wheel
/// (scroll up) ZOOMS IN (a smaller ortho scale), the intuitive direction. The pan is re-derived so
/// `cursor_world` maps to the SAME screen point: for an orthographic camera centred at `pan` with
/// scale `s`, a world point `w` sits at screen offset `(w - pan) / s`. Holding that screen offset
/// constant across the scale change `s -> s'` gives `pan' = w - (w - pan) * (s' / s)`.
///
/// Because the result is an ABSOLUTE target (computed from the CURRENT committed value + the input
/// delta) it is idempotent under the egui multipass re-run (bevy-traps #8): re-running the closure
/// with the same `notches` and the same committed `current_*` yields the same target, not a
/// double-applied one.
#[must_use]
pub(crate) fn cursor_anchored_zoom(
    current_zoom: CanvasZoom,
    current_pan: PreviewPan,
    cursor_world: Vec2,
    notches: f32,
) -> ZoomOutcome {
    let old_scale = *current_zoom;
    // A positive wheel notch zooms IN → a smaller orthographic scale. `ZOOM_STEP^(-notches)`
    // shrinks the scale on scroll-up; `CanvasZoom::scaled` re-clamps to [MIN, MAX] afterwards.
    let factor = ZOOM_STEP.powf(-notches);
    let new_zoom = current_zoom.scaled(factor);
    let new_scale = *new_zoom;
    // Re-anchor the pan so `cursor_world` stays under the cursor across the (clamped) scale change.
    // `pan' = w - (w - pan) * (s'/s)`. Guard the divide (old_scale is always >= MIN_PREVIEW_SCALE
    // so it is never zero, but keep the ratio finite defensively).
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

    /// C4.13 — a positive wheel notch ZOOMS IN (a SMALLER orthographic scale), the intuitive
    /// direction; a negative notch zooms out.
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

    /// C4.13 — the zoom scale is CLAMPED to the fixed system-constant band `[0.25, 4.0]`: many
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

    /// C4.13 — the zoom is CURSOR-ANCHORED: the world point under the cursor stays FIXED across a
    /// zoom. Verify the anchor invariant `(w - pan) / s == (w - pan') / s'` — the cursor's screen
    /// offset (world delta over scale) is unchanged after the zoom.
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

    /// C4.13 — anchoring at the ORIGIN cursor with an origin pan leaves the pan at the origin (no
    /// spurious drift): `pan' = w - (w - pan) * ratio` with `w = pan = 0` is `0` for any ratio.
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
