//! The per-line reflow slide offset ([`LineSlide`]): the animated displacement a line
//! eases through toward its natural flex slot. Split out of the monolithic
//! `components.rs` (GTW-583); the log rationale lives on the parent `components`
//! module.

use bevy::prelude::*;

/// One live combat-log line's UI-space SLIDE state — the animated vertical offset (in logical
/// px) it is currently displaced from its natural flex slot, eased toward `0` each frame by
/// [`slide_combat_log_lines`](super::super::systems::slide_combat_log_lines) (GTW-328 slice B).
///
/// A NAMED grouping component (not a bare `f32`): the line's natural position is its flex-column
/// slot, but when the stack REFLOWS (a new line appends below, or a faded line is removed above)
/// a line would SNAP to its new slot. To slide instead, the line carries this offset, written
/// into its [`Node::top`](bevy::ui::Node) so the rendered position is `slot + offset`. The slide
/// system lerps `offset → 0` (the natural slot) by the tuned `line_lerp_rate` each frame, so the
/// line eases into place. On spawn the offset is seeded to the line's own height (it slides UP
/// into its slot from just below); on a removal the surviving lines' offsets are bumped by the
/// freed height so they hold their old screen position then glide into the gap.
///
/// `current` mutates ONLY through [`ease_toward_target`](Self::ease_toward_target) /
/// [`displace`](Self::displace) (no `DerefMut`). `pub(crate)`: the spawn + update + slide
/// systems build / bump / ease it. Built through [`new`](Self::new).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct LineSlide {
    /// The current vertical offset from the natural slot, in logical px (eased toward `0`).
    current: f32,
}

impl LineSlide {
    /// Build a line-slide seeded `offset` px from its natural slot (typically the line's height,
    /// so a fresh line slides UP into place from just below it).
    #[must_use]
    pub(crate) const fn new(offset: f32) -> Self {
        Self { current: offset }
    }

    /// The current offset (logical px) to write into the line's [`Node::top`](bevy::ui::Node).
    #[must_use]
    pub(crate) const fn current(self) -> f32 {
        self.current
    }

    /// Add `delta` px to the current offset — used on a REFLOW to bump a surviving line by the
    /// freed height so it holds its old screen position before easing back into the gap.
    pub(crate) fn displace(&mut self, delta: f32) {
        self.current += delta;
    }

    /// Ease the current offset toward `0` (the natural slot) by the frame's `factor`
    /// (`0.0..=1.0`, the `1 - exp(-rate * dt)` smoothing weight); snaps to exactly `0` once the
    /// residual is sub-pixel so the line settles cleanly rather than asymptoting forever.
    pub(crate) fn ease_toward_target(&mut self, factor: f32) {
        self.current *= 1.0 - factor.clamp(0.0, 1.0);
        if self.current.abs() < f32::EPSILON {
            self.current = 0.0;
        }
    }
}
