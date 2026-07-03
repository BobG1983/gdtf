//! The map-editor **central-canvas MODEL** — the state-scoped storey + zoom selectors the editor
//! holds while editing (GTW-423 canvas; GTW-500 selectors; egui-swept GTW-512).
//!
//! ## GTW-512: the clean swap off `bevy_ui`
//!
//! The pre-egui canvas was a `bevy_ui` flex-wrapped grid of fixed-px cell nodes inside a `gdtf_ui`
//! scroll list, with a swarm of drive systems (sync / paint / hover-ghost / centre / mouse-wheel
//! zoom / level-nav chrome). The egui swap REPLACES that whole render path with the
//! egui CENTRAL panel (the viewport — stubbed in C1, drawn in C4 / GTW-515). So the `bevy_ui` canvas
//! render machinery is GONE; this module keeps ONLY the two pure MODEL resources the editor's
//! state-scoped lifecycle inserts and the egui viewport (C4) will read:
//!
//! - [`CurrentEditLevel`] — the storey the canvas edits (the GTW-500 C1 level selector),
//! - [`CanvasZoom`] — the viewport zoom factor (the GTW-500 C3 zoom).
//!
//! Both are state-scoped (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` — bevy-traps #1).

use bevy::prelude::*;
use gdtf_battle_sim::{level::GridSize, metric::Level};

/// The minimum canvas zoom factor — cells shrink to a quarter of their base edge. A framework
/// layout const.
const MIN_ZOOM: f32 = 0.25;

/// The maximum canvas zoom factor — cells grow to four times their base edge. A framework layout
/// const.
const MAX_ZOOM: f32 = 4.0;

/// The storey the canvas is currently editing (GTW-500 C1) — the x/y slice the egui viewport draws,
/// paints, and previews the hover ghost on.
///
/// A named newtype over the sim's [`Level`] storey index (no-bare-types: the edited storey is a
/// domain coordinate). PRIVATE inner, derived [`Deref`] to the wrapped [`Level`]; mutated through
/// [`stepped`](CurrentEditLevel::stepped) / [`clamped`](CurrentEditLevel::clamped), which keep the
/// value inside the prefab's storey range so the canvas can never read a slice past the drawable
/// volume. A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), seeded to the ground storey.
#[derive(Resource, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CurrentEditLevel(Level);

impl CurrentEditLevel {
    /// The ground storey (`L0`) — the level the editor opens on (the GTW-423 canvas's old hardcoded
    /// plane, now the seed of the selector).
    #[must_use]
    pub const fn ground() -> Self {
        Self(Level::new(0))
    }

    /// This level stepped by `delta` storeys, CLAMPED to the prefab's `[0, levels-1]` range so the
    /// result is always inside the drawable volume (C1). A step that would leave the range saturates
    /// at the nearest end.
    #[must_use]
    pub fn stepped(self, delta: LevelStep, size: GridSize) -> Self {
        let current = i32::from(*self.0);
        let max = i32::from(*size.levels()).saturating_sub(1);
        let next = (current + delta.delta()).clamp(0, max);
        // `next` is clamped into `[0, max]` where `max < levels <= MAX_LEVELS` (a `u8`), so the
        // `u8` conversion is always in range — no panic, no truncation in practice.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "next is clamped to [0, levels-1] with levels <= MAX_LEVELS (u8), so it fits \
                      a u8 without wrap or sign-flip"
        )]
        let storey = next as u8;
        Self(Level::new(storey))
    }

    /// This level CLAMPED to the prefab's `[0, levels-1]` range — used when the grid shrinks below
    /// the current storey (a size change must never leave the selector pointing past the new
    /// volume).
    #[must_use]
    pub fn clamped(self, size: GridSize) -> Self {
        self.stepped(LevelStep::none(), size)
    }

    /// The wrapped storey index — the [`Level`] the canvas render / paint / ghost read.
    #[must_use]
    pub const fn level(self) -> Level {
        self.0
    }
}

/// A signed level-navigation step in storeys (no-bare-types: a step is a domain delta). `+1` steps
/// up one storey, `-1` down; `0` is the identity used for a re-clamp.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelStep(i32);

impl LevelStep {
    /// Step UP one storey (toward the ceiling).
    #[must_use]
    pub const fn up() -> Self {
        Self(1)
    }

    /// Step DOWN one storey (toward the ground).
    #[must_use]
    pub const fn down() -> Self {
        Self(-1)
    }

    /// No step — the identity used to re-clamp the current level after a grid shrink.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// The signed storey delta.
    const fn delta(self) -> i32 {
        self.0
    }
}

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
