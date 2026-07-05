//! The 3-frame impact animation state machine and the expanding-scale table.

use bevy::prelude::*;
use gdtf_battle_sim::DamageType;

use super::super::{roles::IMPACT_FRAME_COUNT, tuning::ImpactFrameSeconds};

/// The per-frame UNIFORM draw scale of the impact strip, frame-by-frame (GTW-306 V3 fix).
///
/// The sheet's impact strip is an expanding shockwave (dense burst → open ring → breaking
/// ring), but at 1× the three tiles are a similar size, so the expansion barely reads in a
/// captured frame. Drawing each successive frame a little LARGER — uniformly, the SAME factor
/// on both axes (never a one-axis stretch) — makes the burst visibly GROW into a ring, so the
/// impact reads as a 3-frame animation at the arrival point rather than one static blob. One
/// entry per [`IMPACT_FRAME_COUNT`] frame, in play order. A `const` table of uniform
/// multipliers (framework plumbing fed to `custom_size`), not a domain value.
pub(super) const IMPACT_FRAME_SCALES: [f32; IMPACT_FRAME_COUNT] = [1.1, 1.5, 1.9];

/// The UNIFORM draw scale for impact frame `frame` (GTW-306 V3 fix).
///
/// Looks up [`IMPACT_FRAME_SCALES`]`[frame]`, falling back to the LAST entry for an
/// out-of-range frame (a short authored strip degrades to the largest ring rather than
/// snapping to 1×). Keeps the burst → ring growth data-table-driven and panic-free.
pub(super) fn impact_frame_scale(frame: usize) -> f32 {
    IMPACT_FRAME_SCALES
        .get(frame)
        .copied()
        .or_else(|| IMPACT_FRAME_SCALES.last().copied())
        .unwrap_or(1.0)
}

/// One playing impact animation — which of the 3 frames is showing, its per-frame
/// clock, and the damage type that selects the strip.
///
/// A NAMED grouping component (not a bare tuple): `damage` is the shot's
/// [`DamageType`] (so each frame redraws the matching impact tile from
/// [`EffectRoles::fx_for`](super::super::roles::EffectRoles::fx_for)), `frame` is the
/// 0-based index into the `IMPACT_FRAME_COUNT`-long impact strip currently on
/// screen, `frame_seconds` is the per-frame hold this animation CAPTURED from the
/// hot-reloadable [`FxTuning`](super::super::tuning::FxTuning) at spawn (so a live `.ron` edit
/// re-tunes the next
/// impact's pacing), and `clock` is the per-frame [`Timer`]
/// [`animate_impact`](super::animate::animate_impact) ticks.
/// The inner timer + frame mutate ONLY through
/// [`advance`](ImpactAnimation::advance) (no `DerefMut`); construction is through
/// [`new`](ImpactAnimation::new).
#[derive(Component, Debug, Clone)]
pub struct ImpactAnimation {
    /// The shot's damage type — picks which color row's 3-frame impact strip plays.
    damage:        DamageType,
    /// The 0-based index of the impact frame currently shown (`0..IMPACT_FRAME_COUNT`).
    frame:         usize,
    /// The per-frame hold (seconds) this impact CAPTURED from the hot-reloadable
    /// [`FxTuning`](super::super::tuning::FxTuning) at spawn — the duration each `clock` runs
    /// for.
    frame_seconds: ImpactFrameSeconds,
    /// The per-frame hold clock (`*frame_seconds`); each finish steps `frame`.
    clock:         Timer,
}

/// What [`ImpactAnimation::advance`] reports after a tick — the animation either
/// keeps playing (the caller redraws the current frame's tile), or has finished
/// its last frame (the caller despawns it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImpactStep {
    /// The animation advanced to (or held on) frame `index` — redraw that tile.
    Showing(usize),
    /// The last impact frame has finished playing — despawn the animation entity.
    Finished,
}

impl ImpactAnimation {
    /// Start a fresh impact animation for `damage`, holding each frame for
    /// `frame_seconds`, showing frame `0`.
    ///
    /// `frame_seconds` is the hot-reloadable [`ImpactFrameSeconds`] the caller READ from
    /// the resident [`FxTuning`](super::super::tuning::FxTuning) resource — CAPTURED here so a
    /// later `.ron` edit re-tunes
    /// the NEXT impact's pacing rather than a playing one. A [`TimerMode::Once`] per-frame
    /// clock runs for `*frame_seconds`; each time it finishes
    /// [`advance`](ImpactAnimation::advance) steps to the next frame (resetting the clock)
    /// until the last frame elapses.
    #[must_use]
    pub fn new(damage: DamageType, frame_seconds: ImpactFrameSeconds) -> Self {
        Self {
            damage,
            frame: 0,
            frame_seconds,
            clock: Timer::from_seconds(*frame_seconds, TimerMode::Once),
        }
    }

    /// The damage type whose 3-frame impact strip this animation plays.
    #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.damage
    }

    /// Advance the per-frame clock by `delta`, stepping to the next impact frame
    /// when the hold elapses, and report the resulting [`ImpactStep`].
    ///
    /// While the current frame's `*frame_seconds` hold (the captured hot-reloadable
    /// [`ImpactFrameSeconds`]) has not elapsed it returns [`ImpactStep::Showing`] for
    /// the SAME frame (no redraw change). On the frame the clock finishes it steps
    /// `frame` forward and resets the clock: if a next frame exists it returns
    /// [`ImpactStep::Showing`] for it (the caller swaps the sprite to that tile); once
    /// the LAST frame (`IMPACT_FRAME_COUNT - 1`) has finished it returns
    /// [`ImpactStep::Finished`] (the caller despawns). Wraps [`Timer::tick`] so the
    /// inner [`Timer`] + frame mutate only here (no `DerefMut`).
    pub(super) fn advance(&mut self, delta: std::time::Duration) -> ImpactStep {
        if !self.clock.tick(delta).is_finished() {
            // Still holding on the current frame — keep showing it.
            return ImpactStep::Showing(self.frame);
        }
        // This frame's hold elapsed — step to the next.
        self.frame += 1;
        if self.frame >= IMPACT_FRAME_COUNT {
            return ImpactStep::Finished;
        }
        // Reset the one-shot clock for the next frame's hold (the captured tuning value).
        self.clock = Timer::from_seconds(*self.frame_seconds, TimerMode::Once);
        ImpactStep::Showing(self.frame)
    }
}
