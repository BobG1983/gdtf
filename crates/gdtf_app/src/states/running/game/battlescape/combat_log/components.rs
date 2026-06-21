//! Marker + state components for the battlescape combat-text LOG (GTW-328, slice 3,
//! bottom-left, ABOVE the weapon panel).
//!
//! The combat log is a battle-scoped `flex column` of recent-combat-event lines that scroll up
//! and fade. [`CombatLogRoot`] is the anchored container (the `OnExit` despawn + the per-update
//! append both find it by this marker); it also carries [`PanelHeightAnim`] (the GTW-328 slice-B
//! animated height that LERPS toward the natural content height instead of snapping). Each line
//! entity carries [`CombatLogLine`] (so the update can FIFO-despawn the oldest by spawn order +
//! count the visible lines) plus a [`LogLineFade`] holding its three-phase fade clock
//! (fade-in / hold / fade-out — the UI-space rise-free analogue of the presenter's world-space
//! [`FloatingCombatText`](gdtf_battle_presenter) shape) and a [`LineSlide`] holding its animated
//! position offset (the GTW-328 slice-B per-line slide that eases toward its target slot as the
//! stack reflows).
//!
//! UI/view only — these markers carry no combat rule; the lines are built from the sim's
//! combat-event messages via the shared
//! [`classify_log_event`](gdtf_battle_presenter::classify_log_event) classifier.

use bevy::prelude::*;

use crate::states::running::game::battlescape::combat_log::tuning::{
    FadeInSeconds, FadeOutSeconds, LineTtlSeconds,
};

crate::support_item! {
    /// Marks the **root** container of the combat-log tree — the bottom-left `flex column` the
    /// event lines append into, so the `OnExit(BattleRunning)` despawn finds and recursively
    /// tears down the whole log by this one marker, and the per-update append finds the parent
    /// to spawn each new line under.
    ///
    /// Widened toward `crate::test_support` via [`support_item!`](crate::support_item) so the
    /// AC test can name it to assert the log gains line children. A unit marker: presence on an
    /// entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogRoot;
}

crate::support_item! {
    /// Marks one **combat-log line** entity — a UI [`Text`](bevy::prelude::Text) node holding a
    /// classified [`LogLine`](gdtf_battle_presenter::LogLine)'s text, parented under
    /// [`CombatLogRoot`]. The per-update overflow trim finds the visible lines by this marker
    /// (FIFO-despawning the OLDEST when the count exceeds the tuned max), and the AC test names
    /// it to assert the rendered lines + overflow.
    ///
    /// Widened toward `crate::test_support`. A unit marker: presence on an entity is the whole
    /// signal (no-bare-types rule). The line's lifetime / fade state rides the sibling
    /// [`LogLineFade`] component.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogLine;
}

/// One live combat-log line's UI-space three-phase FADE state — its remaining-life clock plus
/// the fade-in / hold / fade-out window boundaries and its base alpha, advanced each frame by
/// [`fade_combat_log_lines`](super::systems::fade_combat_log_lines).
///
/// A NAMED grouping component (not a bare tuple): mirrors the presenter's world-space
/// [`FloatingCombatText`](gdtf_battle_presenter) TTL shape but for a UI text node — there is NO
/// rise (the lines slide via [`LineSlide`], not a `Transform`), only the lifetime clock + the
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

/// One live combat-log line's UI-space SLIDE state — the animated vertical offset (in logical
/// px) it is currently displaced from its natural flex slot, eased toward `0` each frame by
/// [`slide_combat_log_lines`](super::systems::slide_combat_log_lines) (GTW-328 slice B).
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

/// The combat-log panel's ANIMATED HEIGHT state — the current logical-px height it is rendering
/// at, eased each frame toward the natural content height by
/// [`animate_combat_log_height`](super::systems::animate_combat_log_height) (GTW-328 slice B).
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

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::{LineSlide, LogLineFade, PanelHeightAnim};
    use crate::states::running::game::battlescape::combat_log::tuning::{
        FadeInSeconds, FadeOutSeconds, LineTtlSeconds,
    };

    /// A line that is `f32::EPSILON`-close to `expected` (the float-cmp idiom; clippy `float_cmp`
    /// denies `==` on `f32`). A small slack so the `clamp`/`mul_add` rounding does not flake.
    fn approx(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() < 1e-4
    }

    /// Build the three-phase fade for a `ttl`-second line with the given fade-in / fade-out
    /// windows, scaling from full opacity (`1.0`).
    fn fade(ttl: f32, fade_in: f32, fade_out: f32) -> LogLineFade {
        LogLineFade::new(
            LineTtlSeconds::from_secs(ttl),
            FadeInSeconds::from_secs(fade_in),
            FadeOutSeconds::from_secs(fade_out),
            1.0,
        )
    }

    // ---- LogLineFade — the three-phase fade-in / hold / fade-out curve (GTW-328 slice B). ----

    /// On spawn (zero elapsed) a line's alpha is `0` — it FADES IN rather than snapping to full
    /// opacity.
    #[test]
    fn a_fresh_line_starts_transparent_and_fades_in() {
        let f = fade(5.0, 0.2, 1.0);
        assert!(
            approx(f.alpha(), 0.0),
            "alpha at spawn is 0, got {}",
            f.alpha()
        );
    }

    /// Partway through the fade-IN window the alpha is a partial ramp toward full (here halfway
    /// through a `0.2`s fade-in => ~`0.5`).
    #[test]
    fn mid_fade_in_the_alpha_is_a_partial_ramp_up() {
        let mut f = fade(5.0, 0.2, 1.0);
        f.advance(Duration::from_millis(100)); // halfway through the 0.2s fade-in
        assert!(
            approx(f.alpha(), 0.5),
            "halfway through fade-in alpha is ~0.5, got {}",
            f.alpha(),
        );
    }

    /// After the fade-in window and before the fade-out window the line HOLDS at full opacity.
    #[test]
    fn the_hold_phase_is_full_opacity() {
        let mut f = fade(5.0, 0.2, 1.0);
        f.advance(Duration::from_secs_f32(2.0)); // well past fade-in (0.2), well before fade-out
        assert!(
            approx(f.alpha(), 1.0),
            "in the hold phase alpha is full, got {}",
            f.alpha(),
        );
    }

    /// Inside the fade-OUT window (the final `fade_out` seconds) the alpha ramps from full toward
    /// `0` (here halfway through the `1.0`s fade-out => ~`0.5`).
    #[test]
    fn mid_fade_out_the_alpha_is_a_partial_ramp_down() {
        let mut f = fade(5.0, 0.2, 1.0);
        // Fade-out starts at life - fade_out = 4.0s; advance to 4.5s (halfway through the 1.0s tail).
        f.advance(Duration::from_secs_f32(4.5));
        assert!(
            approx(f.alpha(), 0.5),
            "halfway through fade-out alpha is ~0.5, got {}",
            f.alpha(),
        );
    }

    /// Overlapping fade windows (fade-in + fade-out exceed the life) are scaled to FIT — neither
    /// is dropped, and the curve still completes (the fade-out reaches ~0 by end of life).
    #[test]
    fn overlapping_fade_windows_are_scaled_to_fit_the_life() {
        // 1s life, but 0.8 + 0.8 = 1.6s of fades requested -> scaled to 0.5 + 0.5.
        let mut f = fade(1.0, 0.8, 0.8);
        f.advance(Duration::from_secs_f32(0.999)); // essentially end of life
        assert!(
            f.alpha() < 0.1,
            "with windows scaled to fit, the alpha still fades to ~0 by end of life, got {}",
            f.alpha(),
        );
    }

    /// The clock finishes (signals despawn) the frame its TTL elapses.
    #[test]
    fn the_clock_finishes_at_end_of_life() {
        let mut f = fade(0.5, 0.1, 0.1);
        assert!(
            !f.advance(Duration::from_millis(250)),
            "not finished mid-life"
        );
        assert!(
            f.advance(Duration::from_millis(300)),
            "the clock finishes once the TTL has elapsed",
        );
    }

    // ---- LineSlide — the per-line position ease toward the target slot (GTW-328 slice B). ----

    /// A fresh slide carries its seeded appear-offset, and easing toward the target (`0`) STRICTLY
    /// SHRINKS it (never snaps) until it settles at exactly `0`.
    #[test]
    fn a_slide_eases_toward_zero_without_snapping() {
        let mut slide = LineSlide::new(20.0);
        assert!(
            approx(slide.current(), 20.0),
            "the seeded offset is preserved"
        );

        let after_one = {
            slide.ease_toward_target(0.3);
            slide.current()
        };
        assert!(
            after_one < 20.0 && after_one > 0.0,
            "one ease step shrinks the offset toward 0 without snapping, got {after_one}",
        );

        // Many steps settle it to exactly 0 (the sub-pixel snap).
        for _ in 0..200 {
            slide.ease_toward_target(0.3);
        }
        assert!(
            approx(slide.current(), 0.0),
            "the slide settles at exactly 0, got {}",
            slide.current(),
        );
    }

    /// A reflow DISPLACES the offset by the freed height — the survivor holds its old screen
    /// position (a bigger offset) before easing back into the gap.
    #[test]
    fn a_reflow_displaces_the_slide_by_the_freed_height() {
        let mut slide = LineSlide::new(0.0); // already settled
        slide.displace(15.0); // a line above was removed; bump by its height
        assert!(
            approx(slide.current(), 15.0),
            "the freed height is added to the offset, got {}",
            slide.current(),
        );
    }

    // ---- PanelHeightAnim — the root height ease toward the content height (GTW-328 slice B). ----

    /// The panel height GROWS toward a taller target as lines are added (eased, not snapped) and
    /// settles at exactly the target.
    #[test]
    fn the_panel_height_grows_toward_a_taller_target() {
        let mut anim = PanelHeightAnim::new(0.0);
        anim.ease_toward(100.0, 0.25);
        let after_one = anim.current();
        assert!(
            after_one > 0.0 && after_one < 100.0,
            "one ease step grows toward the target without snapping, got {after_one}",
        );
        for _ in 0..200 {
            anim.ease_toward(100.0, 0.25);
        }
        assert!(
            approx(anim.current(), 100.0),
            "the height settles at exactly the target, got {}",
            anim.current(),
        );
    }

    /// The panel height SHRINKS toward a shorter target as lines are removed (eased, not snapped).
    #[test]
    fn the_panel_height_shrinks_toward_a_shorter_target() {
        let mut anim = PanelHeightAnim::new(100.0);
        anim.ease_toward(40.0, 0.25);
        let after_one = anim.current();
        assert!(
            after_one < 100.0 && after_one > 40.0,
            "one ease step shrinks toward the shorter target without snapping, got {after_one}",
        );
        for _ in 0..200 {
            anim.ease_toward(40.0, 0.25);
        }
        assert!(
            approx(anim.current(), 40.0),
            "the height settles at exactly the shorter target, got {}",
            anim.current(),
        );
    }
}
