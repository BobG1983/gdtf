//! The per-line three-phase fade clock ([`LogLineFade`]): fade-in / hold / fade-out
//! over a line's TTL. Split out of the monolithic `components.rs` (GTW-583); the log
//! rationale lives on the parent `components` module.

use bevy::prelude::*;

use crate::states::running::game::battlescape::combat_log::tuning::{
    FadeInSeconds, FadeOutSeconds, LineTtlSeconds,
};

/// One live combat-log line's UI-space three-phase FADE state — its remaining-life clock plus
/// the fade-in / hold / fade-out window boundaries and its base alpha, advanced each frame by
/// [`fade_combat_log_lines`](super::super::systems::fade_combat_log_lines).
///
/// A NAMED grouping component (not a bare tuple): mirrors the presenter's world-space
/// [`FloatingCombatText`](gdtf_battle_presenter) TTL shape but for a UI text node — there is NO
/// rise (the lines slide via [`LineSlide`](super::slide::LineSlide), not a `Transform`), only the lifetime clock + the
/// three-phase alpha curve (GTW-328 slice B):
///
/// 1. **fade in** — `0 → base_alpha` over the first `fade_in` seconds (a gentle entrance, not a
///    snap-on),
/// 2. **hold** — `base_alpha` through the middle of the life,
/// 3. **fade out** — `base_alpha → 0` over the final `fade_out` seconds, despawning the frame
///    the clock finishes.
///
/// The clock ticks each frame; it mutates ONLY through [`advance`](Self::advance) (no
/// `DerefMut`). `base_alpha` is captured at spawn so the curve scales from the line's starting
/// opacity. The windows are stored in SECONDS (clamped so they fit the TTL), so a long absolute
/// fade-out still completes before the line despawns rather than being cut off.
///
/// `pub(crate)`: the spawn system builds it and the fade system reads/advances it; the AC test
/// does not name it (it asserts on the rendered text + count). Built through [`new`](Self::new).
#[derive(Component, Debug, Clone)]
pub(crate) struct LogLineFade {
    /// The one-shot remaining-life clock; the line despawns the frame it finishes.
    ttl:        Timer,
    /// The total lifetime in seconds (the clock's duration), cached so the fade windows can be
    /// compared against the elapsed seconds.
    life:       f32,
    /// How long the fade-IN ramp lasts, in seconds from spawn (clamped to fit the life).
    fade_in:    f32,
    /// How long the fade-OUT ramp lasts, in seconds before end of life (clamped to fit the
    /// life after the fade-in).
    fade_out:   f32,
    /// The line's TARGET alpha (full opacity) — fade-in ramps up to it and fade-out ramps down
    /// from it. Captured at spawn so the curve scales from the line's starting opacity.
    base_alpha: f32,
}

impl LogLineFade {
    /// Build a three-phase line-fade clock for a line living `ttl` seconds: fade in over
    /// `fade_in`, hold, then fade out over `fade_out`, scaling from `base_alpha`.
    ///
    /// The two fade windows are clamped to NON-NEGATIVE and, if their sum exceeds the life,
    /// scaled down proportionally so they always fit (a malformed tuning value cannot invert
    /// the curve or push the fade-out past the despawn). `base_alpha` is the line's spawn / full
    /// opacity (typically `1.0`).
    #[must_use]
    pub(crate) fn new(
        ttl: LineTtlSeconds,
        fade_in: FadeInSeconds,
        fade_out: FadeOutSeconds,
        base_alpha: f32,
    ) -> Self {
        let life = (*ttl).max(0.0);
        let mut fade_in = (*fade_in).max(0.0);
        let mut fade_out = (*fade_out).max(0.0);
        // If the two ramps overlap (their sum exceeds the life), scale both down proportionally
        // so fade-in + fade-out fit within the life with no hold rather than crossing over.
        let total = fade_in + fade_out;
        if total > life && total > f32::EPSILON {
            let scale = life / total;
            fade_in *= scale;
            fade_out *= scale;
        }
        Self {
            ttl: Timer::from_seconds(life, TimerMode::Once),
            life,
            fade_in,
            fade_out,
            base_alpha,
        }
    }

    /// Advance the lifetime clock by `delta`; returns `true` the frame the clock finishes (the
    /// line should despawn).
    pub(crate) fn advance(&mut self, delta: std::time::Duration) -> bool {
        self.ttl.tick(delta).is_finished()
    }

    /// The line's current alpha along the three-phase curve — a `0 → base_alpha` ramp over the
    /// fade-in window, a hold at `base_alpha`, then a `base_alpha → 0` ramp over the fade-out
    /// window at end of life.
    #[must_use]
    pub(crate) fn alpha(&self) -> f32 {
        let elapsed = self.ttl.elapsed_secs();
        // Fade IN: 0 -> base_alpha over the first `fade_in` seconds.
        if elapsed < self.fade_in && self.fade_in > f32::EPSILON {
            return (self.base_alpha * (elapsed / self.fade_in)).clamp(0.0, self.base_alpha);
        }
        // Fade OUT: base_alpha -> 0 over the final `fade_out` seconds.
        let fade_out_start = self.life - self.fade_out;
        if elapsed >= fade_out_start && self.fade_out > f32::EPSILON {
            let into_fade = (elapsed - fade_out_start) / self.fade_out;
            return (self.base_alpha * (1.0 - into_fade)).clamp(0.0, self.base_alpha);
        }
        // Hold at full opacity in the middle.
        self.base_alpha
    }
}
