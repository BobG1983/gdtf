//! Unit tests for the §8 reaction-fire layer.
//!
//! Two slices, all value-agnostic (assert structural / relative invariants, never
//! pinned tunable magnitudes — the GTW-466 test style):
//!
//! - GTW-466 leaf invariants ([`super::leaves`]): cap monotonicity + clamp edges.
//! - GTW-467 opposed-check core ([`super::core`]): determinism (C6a), monotonicity
//!   (C6b), clamp at extremes (C6c), per-turn cap (C6d), degenerate cases (C6e).

use super::{
    core::{
        ReactionProbability, ReactionScore, ReactionsUsed, interrupt_probability, may_interrupt,
        reaction_score, rolls_interrupt,
    },
    leaves::{ReactionTuning, clamp_probability, reaction_cap},
};
use crate::{
    ganger::{Reactions, Tu, TuMax},
    rng::{BattleSeed, ReactionRng},
};

// ── GTW-466 leaf invariants ──────────────────────────────────────────────────

/// The cap is **monotone non-decreasing** in [`Reactions`]: a ganger with
/// higher Reactions never gets fewer interrupts than one with lower Reactions
/// (the consistency invariant from the ticket contract).
///
/// Value-agnostic: never pins the default magnitudes — only the ordering
/// relation across a sequence of ascending Reactions values.
#[test]
fn reaction_cap_is_monotone_nondecreasing_in_reactions() {
    let tuning = ReactionTuning::default();
    // Ascending sequence of Reactions values (arbitrary, never shipped defaults).
    let values = [0.0_f32, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 20.0];
    let caps: Vec<u32> = values
        .iter()
        .map(|&r| *reaction_cap(Reactions::new(r), &tuning))
        .collect();

    for window in caps.windows(2) {
        let (prev, next) = (window[0], window[1]);
        assert!(
            prev <= next,
            "reaction_cap must be monotone non-decreasing in Reactions: \
             cap({prev}) > cap({next}) — invariant violated",
        );
    }
}

/// The clamp maps a value BELOW `p_min` UP to `p_min`.
#[test]
fn clamp_probability_lifts_below_p_min_to_p_min() {
    let tuning = ReactionTuning::default();
    // Arbitrary value strictly below the default p_min.
    let below = *tuning.p_min - 0.02;
    let clamped = clamp_probability(ReactionProbability::new(below), &tuning);
    assert_eq!(
        clamped.to_bits(),
        (*tuning.p_min).to_bits(),
        "a value below p_min must be clamped up to p_min",
    );
}

/// The clamp maps a value ABOVE `p_max` DOWN to `p_max`.
#[test]
fn clamp_probability_lowers_above_p_max_to_p_max() {
    let tuning = ReactionTuning::default();
    // Arbitrary value strictly above the default p_max.
    let above = *tuning.p_max + 0.02;
    let clamped = clamp_probability(ReactionProbability::new(above), &tuning);
    assert_eq!(
        clamped.to_bits(),
        (*tuning.p_max).to_bits(),
        "a value above p_max must be clamped down to p_max",
    );
}

/// The clamp leaves an IN-RANGE value unchanged.
#[test]
fn clamp_probability_passes_through_in_range_value() {
    let tuning = ReactionTuning::default();
    // An arbitrary in-range value (midpoint between p_min and p_max).
    let mid = f32::midpoint(*tuning.p_min, *tuning.p_max);
    let clamped = clamp_probability(ReactionProbability::new(mid), &tuning);
    assert_eq!(
        clamped.to_bits(),
        mid.to_bits(),
        "a value already in [p_min, p_max] must pass through unchanged",
    );
}

// ── GTW-467 C6a — rolls_interrupt deterministic-replayable ───────────────────

/// Number of consecutive interrupt rolls compared when asserting replay
/// determinism (mirrors the rng `STREAM_LEN` pattern).
const ROLL_LEN: usize = 64;

/// C6a: two [`ReactionRng`]s built from the SAME seed, fed the SAME probability,
/// produce an IDENTICAL `bool` interrupt sequence — deterministic-replayable
/// (mirrors `same_seed_same_stream`). Each roll is exactly one draw, so the two
/// streams advance in lockstep.
#[test]
fn rolls_interrupt_is_deterministic_replayable_under_same_seed() {
    let tuning = ReactionTuning::default();
    // An arbitrary in-range probability (the midpoint, well inside the clamp).
    let p = interrupt_probability(ReactionScore::new(3.0), ReactionScore::new(3.0), &tuning);

    // An arbitrary fixed seed — determinism is the property, the value is irrelevant.
    let mut a = ReactionRng::from_root(BattleSeed::new(0x0467_C0DE_DEAD_BEEF));
    let mut b = ReactionRng::from_root(BattleSeed::new(0x0467_C0DE_DEAD_BEEF));

    let seq_a: Vec<bool> = (0..ROLL_LEN).map(|_| *rolls_interrupt(p, &mut a)).collect();
    let seq_b: Vec<bool> = (0..ROLL_LEN).map(|_| *rolls_interrupt(p, &mut b)).collect();

    assert_eq!(
        seq_a, seq_b,
        "same seed + same probability must yield an identical interrupt sequence",
    );
}

// ── GTW-467 C6b — monotonicity / correctness ─────────────────────────────────

/// C6b: a HIGHER watcher score yields a STRICTLY higher `P(interrupt)` than a
/// lower one, holding the mover fixed. Pin-discriminating — FAILS if the ratio
/// is inverted. Chosen scores keep both raw probabilities strictly inside the
/// `[p_min, p_max]` clamp so the strict inequality survives the clamp.
#[test]
fn higher_watcher_score_yields_strictly_higher_probability() {
    let tuning = ReactionTuning::default();
    let mover = ReactionScore::new(4.0);
    // Two watcher scores either side of the mover, both giving an in-clamp ratio.
    let p_low = interrupt_probability(ReactionScore::new(3.0), mover, &tuning);
    let p_high = interrupt_probability(ReactionScore::new(5.0), mover, &tuning);
    assert!(
        *p_high > *p_low,
        "a higher watcher score must yield a strictly higher P(interrupt): \
         p_high={} !> p_low={}",
        *p_high,
        *p_low,
    );
}

/// C6b: a mover spending MORE TU (a LOWER `tu_left`) LOWERS the mover's score
/// and RAISES the watcher's `P(interrupt)`. Pin-discriminating — FAILS if
/// `reaction_score` ignores `tu_left` (the scores would be equal and the two
/// probabilities identical).
#[test]
fn mover_spending_more_tu_lowers_its_score_and_raises_watcher_probability() {
    let tuning = ReactionTuning::default();
    let reactions = Reactions::new(4.0);
    let tu_max = TuMax::new(10);

    // A fresh mover (full TU) vs a mover who has spent TU (lower tu_left).
    let mover_full = reaction_score(reactions, Tu::new(8), tu_max);
    let mover_spent = reaction_score(reactions, Tu::new(2), tu_max);
    assert!(
        *mover_spent < *mover_full,
        "spending TU must lower the mover's score: spent={} !< full={}",
        *mover_spent,
        *mover_full,
    );

    // A fixed watcher; the same watcher interrupts the depleted mover more often.
    let watcher = reaction_score(reactions, Tu::new(6), tu_max);
    let p_vs_full = interrupt_probability(watcher, mover_full, &tuning);
    let p_vs_spent = interrupt_probability(watcher, mover_spent, &tuning);
    assert!(
        *p_vs_spent > *p_vs_full,
        "a mover who spent more TU must be easier to interrupt: \
         p_vs_spent={} !> p_vs_full={}",
        *p_vs_spent,
        *p_vs_full,
    );
}

// ── GTW-467 C6c — clamp at extremes ──────────────────────────────────────────

/// C6c: an overwhelming WATCHER (mover score ~0) clamps `P` to `p_max` — strictly
/// `< 1.0`, never an absolute 100%. And an overwhelming MOVER clamps `P` to
/// `p_min` — strictly `> 0.0`, never an absolute 0%. Value-agnostic: asserts the
/// strict-inside-`(0, 1)` invariant and equality to the clamp edges, not the
/// magnitudes of those edges.
#[test]
fn extremes_clamp_strictly_inside_zero_and_one() {
    let tuning = ReactionTuning::default();

    // Overwhelming watcher: huge watcher score, zero mover score → raw ratio ~1.0.
    let p_overwhelming_watcher =
        interrupt_probability(ReactionScore::new(1000.0), ReactionScore::new(0.0), &tuning);
    assert!(
        *p_overwhelming_watcher < 1.0,
        "an overwhelming watcher must clamp strictly below 1.0, got {}",
        *p_overwhelming_watcher,
    );
    assert_eq!(
        p_overwhelming_watcher.to_bits(),
        (*tuning.p_max).to_bits(),
        "an overwhelming watcher must clamp to p_max",
    );

    // Overwhelming mover: zero watcher score, huge mover score → raw ratio ~0.0.
    let p_overwhelming_mover =
        interrupt_probability(ReactionScore::new(0.0), ReactionScore::new(1000.0), &tuning);
    assert!(
        *p_overwhelming_mover > 0.0,
        "an overwhelming mover must clamp strictly above 0.0, got {}",
        *p_overwhelming_mover,
    );
    assert_eq!(
        p_overwhelming_mover.to_bits(),
        (*tuning.p_min).to_bits(),
        "an overwhelming mover must clamp to p_min",
    );
}

// ── GTW-467 C6d — per-turn cap ───────────────────────────────────────────────

/// C6d: once [`ReactionsUsed`] reaches [`reaction_cap`], [`may_interrupt`]
/// returns `false`; after [`ReactionsUsed::reset`] it returns `true` again (the
/// turn-boundary reset). Value-agnostic: derives the cap from the function, never
/// pins a magnitude.
#[test]
fn cap_blocks_at_limit_and_reset_reopens() {
    let tuning = ReactionTuning::default();
    let reactions = Reactions::new(4.0);
    let cap = *reaction_cap(reactions, &tuning);

    // Below the cap, interrupts are allowed.
    let used_below = ReactionsUsed::new(cap.saturating_sub(1));
    assert!(
        *may_interrupt(used_below, reactions, &tuning),
        "with used < cap, may_interrupt must be true",
    );

    // Drive a fresh counter up to the cap via increment(); at the cap it blocks.
    let mut used = ReactionsUsed::default();
    for _ in 0..cap {
        assert!(
            *may_interrupt(used, reactions, &tuning),
            "every interrupt up to the cap must be allowed",
        );
        used.increment();
    }
    assert_eq!(*used, cap, "increment must reach exactly the cap");
    assert!(
        !*may_interrupt(used, reactions, &tuning),
        "at the cap, may_interrupt must be false (locked out for the turn)",
    );

    // The turn boundary resets the counter → interrupts re-open.
    used.reset();
    assert_eq!(*used, 0, "reset must zero the counter");
    assert!(
        *may_interrupt(used, reactions, &tuning),
        "after reset, may_interrupt must be true again",
    );
}

// ── GTW-467 C6e — degenerate cases (no NaN/panic) ────────────────────────────

/// C6e: [`reaction_score`] with `tu_max == 0` returns `ReactionScore(0.0)` — no
/// division-by-zero NaN/inf, no panic. Asserts finiteness AND the exact `0.0`.
#[test]
fn reaction_score_with_zero_tu_max_is_zero_and_finite() {
    let score = reaction_score(Reactions::new(5.0), Tu::new(3), TuMax::new(0));
    assert!(
        score.is_finite(),
        "reaction_score with tu_max==0 must be finite, got {}",
        *score,
    );
    // Exact-zero check via bit pattern (avoids the float_cmp lint; the defined
    // value is exactly 0.0, never an approximation).
    assert_eq!(
        score.to_bits(),
        0.0_f32.to_bits(),
        "reaction_score with tu_max==0 must be exactly 0.0 (no div-by-zero)",
    );
}

/// C6e: [`interrupt_probability`] with a zero denominator (both scores 0.0)
/// returns the DEFINED clamped fallback — no `0/0` NaN, no panic. Asserts
/// finiteness AND equality to `clamp_probability(0.5, tuning)` (the documented
/// even-odds fallback).
#[test]
fn interrupt_probability_with_zero_denominator_is_defined_and_finite() {
    let tuning = ReactionTuning::default();
    let p = interrupt_probability(ReactionScore::new(0.0), ReactionScore::new(0.0), &tuning);
    assert!(
        p.is_finite(),
        "interrupt_probability with a zero denominator must be finite, got {}",
        *p,
    );
    assert_eq!(
        p.to_bits(),
        clamp_probability(ReactionProbability::new(0.5), &tuning).to_bits(),
        "the zero-denominator fallback must be clamp_probability(0.5, tuning)",
    );
}
