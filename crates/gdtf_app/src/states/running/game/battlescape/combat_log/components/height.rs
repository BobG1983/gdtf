//! The animated panel height ([`PanelHeightAnim`]): the eased logical-px height the
//! combat-log root renders at. Split out of the monolithic `components.rs` (GTW-583);
//! the log rationale lives on the parent `components` module.

use bevy::prelude::*;

/// The combat-log panel's ANIMATED HEIGHT state — the current logical-px height it is rendering
/// at, eased each frame toward the natural content height by
/// [`animate_combat_log_height`](super::super::systems::animate_combat_log_height) (GTW-328 slice B).
///
/// A NAMED grouping component (not a bare `f32`): the panel was `height: Auto` (it SNAPPED to
/// fit its lines). To grow/shrink smoothly the root carries this animated height, written into
/// its [`Node::height`](bevy::ui::Node) as `Val::Px(current)`. The height system measures the
/// natural content height (the summed [`ComputedNode`](bevy::ui::ComputedNode) heights of the
/// visible lines) and lerps `current → target` by the tuned `height_lerp_rate` each frame — up
/// as lines are added, down as they fade / are removed.
///
/// `current` mutates ONLY through [`ease_toward`](Self::ease_toward) (no `DerefMut`).
/// `pub(crate)`: the spawn system seeds it (at `0`, so the first lines grow in) and the height
/// system eases it. Built through [`new`](Self::new).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PanelHeightAnim {
    /// The current rendered height, in logical px (eased toward the natural content height).
    current: f32,
}

impl PanelHeightAnim {
    /// Build a panel-height animator at `current` logical px (the spawn seeds `0.0` so the panel
    /// grows in from nothing as its first lines appear).
    #[must_use]
    pub(crate) const fn new(current: f32) -> Self {
        Self { current }
    }

    /// The current animated height (logical px) to write into the root's
    /// [`Node::height`](bevy::ui::Node).
    #[must_use]
    pub(crate) const fn current(self) -> f32 {
        self.current
    }

    /// Ease the current height toward `target` px by the frame's `factor` (`0.0..=1.0`, the
    /// `1 - exp(-rate * dt)` smoothing weight); snaps to exactly `target` once the residual is
    /// sub-pixel so the panel settles cleanly.
    pub(crate) fn ease_toward(&mut self, target: f32, factor: f32) {
        self.current = (target - self.current).mul_add(factor.clamp(0.0, 1.0), self.current);
        if (target - self.current).abs() < f32::EPSILON {
            self.current = target;
        }
    }
}
