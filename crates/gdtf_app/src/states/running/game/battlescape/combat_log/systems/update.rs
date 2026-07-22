//! The combat-log per-frame ANIMATIONS (GTW-328 slice B): the three-phase line fade, the
//! slide-into-slot ease, and the panel-height ease.
//!
//! (The event drain + append + FIFO trim moved to the GTW-572 forwarder → appender pipeline:
//! [`forward`](super::forward) writes resolved
//! [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent)s, [`append`](super::append)
//! drains them into lines. This module owns only the mutate-in-place animations.)
//!
//! [`fade_combat_log_lines`] ticks each line's [`LogLineFade`] clock, fades its
//! [`TextColor`](bevy::text::TextColor) alpha across the three-phase curve, and despawns
//! the line the frame its clock finishes (the UI-space analogue of the presenter's
//! world-space `animate_floating_text`). [`slide_combat_log_lines`] eases each line's
//! [`LineSlide`] offset toward its slot. [`animate_combat_log_height`] eases the panel
//! toward its content height. All three run unguarded — each self-gates on its entities
//! existing (an empty query is a no-op).

use bevy::{
    color::Alpha,
    prelude::*,
    text::TextColor as UiTextColor,
    ui::{ComputedNode, Node, Val},
};

use crate::states::running::game::battlescape::combat_log::{
    components::{CombatLogLine, CombatLogRoot, LineSlide, LogLineFade, PanelHeightAnim},
    tuning::CombatLogTuning,
};

/// Tick each combat-log line's three-phase fade clock; set its alpha along the fade-in /
/// hold / fade-out curve and despawn it the frame the clock finishes (GTW-328 slice B).
///
/// Runs in `Update`, unguarded — it self-gates on the [`LogLineFade`] lines existing (an
/// empty query is a no-op). Param-only (`bevy-traps.md` #7): [`Commands`], the [`Time`]
/// read, and the line query.
pub(in crate::states::running::game::battlescape) fn fade_combat_log_lines(
    mut commands: Commands,
    time: Res<Time>,
    mut lines: Query<(Entity, &mut UiTextColor, &mut LogLineFade)>,
) {
    let delta = time.delta();
    for (entity, mut color, mut fade) in &mut lines {
        if fade.advance(delta) {
            commands.entity(entity).despawn();
            continue;
        }
        // Set the alpha along the three-phase curve (fade-in / hold / fade-out), in place.
        color.0.set_alpha(fade.alpha());
    }
}

/// The per-second LERP rate above which the frame smoothing weight is treated as instant —
/// guards the `1 - exp(-rate * dt)` smoothing from a pathological tuning value. A
/// framework-plumbing `const` (the carve-out), not a domain value.
const MAX_LERP_RATE: f32 = 1_000.0;

/// The frame smoothing weight for an exponential ease at `rate` per second over `delta`
/// seconds: `1 - exp(-rate * dt)`, clamped to `0.0..=1.0`. Frame-rate independent — the
/// same `rate` settles in the same wall-clock time at any frame rate. A `rate` of `0` (or
/// negative) yields `0` (no movement); a very large `rate` saturates toward `1`.
fn lerp_factor(rate: f32, delta: f32) -> f32 {
    let rate = rate.clamp(0.0, MAX_LERP_RATE);
    (-rate * delta.max(0.0))
        .exp()
        .mul_add(-1.0, 1.0)
        .clamp(0.0, 1.0)
}

/// Ease each combat-log line's [`LineSlide`] offset toward its target slot (`0`) and write
/// the current offset into the line's [`Node::top`](bevy::ui::Node), so a line slides into
/// place as the stack reflows instead of SNAPPING (GTW-328 slice B).
///
/// Runs in `Update`, unguarded — it self-gates on the [`LineSlide`] lines existing. The
/// offset eases by [`lerp_factor`] at the tuned
/// [`line_lerp_rate`](CombatLogTuning::line_lerp_rate) per frame; the tuning is DEFAULTED
/// if its async RON has not resolved yet. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::game::battlescape) fn slide_combat_log_lines(
    time: Res<Time>,
    tuning: Option<Res<CombatLogTuning>>,
    mut lines: Query<(&mut Node, &mut LineSlide), With<CombatLogLine>>,
) {
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);
    let factor = lerp_factor(*tuning.line_lerp_rate, time.delta_secs());
    for (mut node, mut slide) in &mut lines {
        slide.ease_toward_target(factor);
        // Write the eased offset into the line's relative `top` (its displacement from its slot).
        node.top = Val::Px(slide.current());
    }
}

/// Ease the combat-log panel's [`PanelHeightAnim`] toward its natural content height (the
/// summed logical line heights plus any active slide displacement plus the bottom
/// clearance) and write it into the root's [`Node::height`](bevy::ui::Node), so the panel
/// grows / shrinks SMOOTHLY as lines are added / removed instead of snapping (GTW-328
/// slice B).
///
/// Runs in `Update`, unguarded — it self-gates on the [`PanelHeightAnim`] root existing. It
/// sums the logical heights of the current [`CombatLogLine`] [`ComputedNode`]s and ADDS the
/// largest active downward [`LineSlide`] offset — so while a fresh line is still sliding UP
/// into place the clipped panel is tall enough to show it fully. It THEN adds a fixed
/// BOTTOM CLEARANCE (the tuned
/// [`bottom_clearance_lines`](CombatLogTuning::bottom_clearance_lines) times the line
/// height) so the panel is always a touch taller than its content — the top-aligned flex
/// column leaves that surplus at the BOTTOM, lifting the NEWEST line's full glyph box above
/// the clip / bottom-bar edge (the cut-off fix; kept in the height target rather than
/// `Node::padding` because `apply_theme` re-derives the themed panel's padding every
/// repaint). Eased by [`lerp_factor`] at the tuned
/// [`height_lerp_rate`](CombatLogTuning::height_lerp_rate). Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::game::battlescape) fn animate_combat_log_height(
    time: Res<Time>,
    tuning: Option<Res<CombatLogTuning>>,
    lines: Query<(&ComputedNode, &LineSlide), With<CombatLogLine>>,
    mut roots: Query<(&mut Node, &mut PanelHeightAnim), With<CombatLogRoot>>,
) {
    let Ok((mut node, mut anim)) = roots.single_mut() else {
        // No log root (not in a battle / not spawned yet) — nothing to animate.
        return;
    };
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);
    // The natural content height: the sum of the visible lines' LOGICAL heights.
    let content: f32 = lines
        .iter()
        .map(|(computed, _)| computed.size().y * computed.inverse_scale_factor())
        .sum();
    // The largest active downward slide offset — a line still easing UP into place is
    // displaced below its slot by this much, so the clipped panel must be this much taller
    // to show it fully. Settles to 0 as the slides finish.
    let max_slide: f32 = lines
        .iter()
        .map(|(_, slide)| slide.current().max(0.0))
        .fold(0.0_f32, f32::max);
    // The fixed bottom clearance below the NEWEST line (a line-height multiple of the
    // responsive line font size) — so the top-aligned column leaves room at the bottom and
    // the bottommost line's full glyph box clears the clip / bottom-bar edge.
    let clearance = *tuning.line_font_pt * *tuning.bottom_clearance_lines;
    let target = height_target(content, max_slide, clearance);

    let factor = lerp_factor(*tuning.height_lerp_rate, time.delta_secs());
    anim.ease_toward(target, factor);
    node.height = Val::Px(anim.current());
}

/// The combat-log panel's animated-height TARGET (logical px): the summed line `content`
/// height, plus the largest active slide displacement `max_slide` (so a still-sliding line
/// is shown fully), plus a bottom `clearance` so the bottommost line clears the panel clip
/// / bottom-bar edge (the cut-off fix).
///
/// The clearance is applied ONLY when there is content (`content > 0`) so an EMPTY log
/// collapses to `0` rather than holding open a `clearance`-tall sliver of empty panel.
fn height_target(content: f32, max_slide: f32, clearance: f32) -> f32 {
    if content <= f32::EPSILON {
        // An empty log animates back to a collapsed 0 height — no clearance sliver held open.
        return 0.0;
    }
    content + max_slide + clearance
}

#[cfg(test)]
mod test {
    use super::height_target;

    /// A line that is close to `expected` (the float-cmp idiom; clippy `float_cmp` denies
    /// `==` on `f32`).
    fn approx(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() < 1e-4
    }

    /// With content present, the height target STRICTLY EXCEEDS the content sum by the
    /// bottom clearance (plus any slide) — the cut-off fix: the panel is taller than its
    /// lines so the top-aligned column leaves room below the newest line for its
    /// descenders to clear the clip.
    #[test]
    fn the_target_reserves_bottom_clearance_above_the_content_sum() {
        let content = 80.0;
        let clearance = 10.0;
        // Settled (no active slide): the target is the content PLUS the clearance.
        let target = height_target(content, 0.0, clearance);
        assert!(
            target > content,
            "with a positive clearance the panel must be taller than its content ({target} \
             must exceed {content}) so the bottommost line is not clipped",
        );
        assert!(
            approx(target, content + clearance),
            "the settled target is exactly content + clearance, got {target}",
        );
    }

    /// The clearance is ADDED ON TOP of an active slide displacement (they do not replace
    /// each other): while a fresh line still slides up, the panel reserves BOTH the slide
    /// room and the bottom clearance.
    #[test]
    fn the_clearance_stacks_with_an_active_slide() {
        let target = height_target(80.0, 6.0, 10.0);
        assert!(
            approx(target, 96.0),
            "content + slide + clearance are all reserved (80 + 6 + 10), got {target}",
        );
    }

    /// An EMPTY log (no content) collapses to `0` — the clearance is NOT held open as a
    /// sliver of empty panel, so an empty log animates fully closed.
    #[test]
    fn an_empty_log_collapses_to_zero_with_no_clearance_sliver() {
        let target = height_target(0.0, 0.0, 10.0);
        assert!(
            approx(target, 0.0),
            "an empty log targets 0 height (no clearance sliver), got {target}",
        );
    }
}
