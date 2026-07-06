//! The GTW-500 C3 zoom factor — [`CanvasZoom`] and its private clamp bounds.

use bevy::prelude::*;

/// The minimum canvas zoom factor — cells shrink to a quarter of their base edge. A framework
/// layout const.
const MIN_ZOOM: f32 = 0.25;

/// The maximum canvas zoom factor — cells grow to four times their base edge. A framework layout
/// const.
const MAX_ZOOM: f32 = 4.0;

/// The canvas **zoom factor** (GTW-500 C3) — the multiplier the GTW-515 preview reuses as the
/// preview camera's `OrthographicProjection::scale` (the `bevy_ui` fixed-px cell edge it once
/// multiplied — `CANVAS_CELL_PX` — died with the `bevy_ui` canvas; GTW-577 C7 deleted it).
///
/// A named newtype over the bare `f32` factor (no-bare-types: a zoom factor is a domain value).
/// PRIVATE inner, derived [`Deref`]; mutated through [`scaled`](CanvasZoom::scaled) /
/// [`reset`](CanvasZoom::reset), which CLAMP the factor to `[MIN_ZOOM, MAX_ZOOM]` so the cells can
/// never be sized to zero or absurdly large. A state-scoped [`Resource`] (inserted
/// `OnEnter(Editing)`, removed `OnExit(Editing)` — bevy-traps #1), seeded to `1.0` (the GTW-423
/// base scale).
#[derive(Resource, Deref, Clone, Copy, PartialEq, Debug)]
pub struct CanvasZoom(f32);

impl CanvasZoom {
    /// The unzoomed factor — the editor's open state.
    #[must_use]
    pub const fn identity() -> Self {
        Self(1.0)
    }

    /// This factor MULTIPLIED by `factor`, CLAMPED to `[MIN_ZOOM, MAX_ZOOM]` (C3). A multiply (not
    /// an add) makes each wheel notch a constant proportional step.
    #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self((self.0 * factor).clamp(MIN_ZOOM, MAX_ZOOM))
    }

    /// Reset to the unzoomed factor — the zoom-reset target (C3).
    #[must_use]
    pub const fn reset() -> Self {
        Self::identity()
    }
}
