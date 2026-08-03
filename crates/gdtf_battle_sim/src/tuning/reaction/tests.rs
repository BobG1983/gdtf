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


#[test]
fn reaction_cap_is_monotone_nondecreasing_in_reactions() {
    let tuning = ReactionTuning::default();
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

#[test]
fn clamp_probability_lifts_below_p_min_to_p_min() {
    let tuning = ReactionTuning::default();
    let below = *tuning.p_min - 0.02;
    let clamped = clamp_probability(ReactionProbability::new(below), &tuning);
    assert_eq!(
        clamped.to_bits(),
        (*tuning.p_min).to_bits(),
        "a value below p_min must be clamped up to p_min",
    );
}

#[test]
fn clamp_probability_lowers_above_p_max_to_p_max() {
    let tuning = ReactionTuning::default();
    let above = *tuning.p_max + 0.02;
    let clamped = clamp_probability(ReactionProbability::new(above), &tuning);
    assert_eq!(
        clamped.to_bits(),
        (*tuning.p_max).to_bits(),
        "a value above p_max must be clamped down to p_max",
    );
}

#[test]
fn clamp_probability_passes_through_in_range_value() {
    let tuning = ReactionTuning::default();
    let mid = f32::midpoint(*tuning.p_min, *tuning.p_max);
    let clamped = clamp_probability(ReactionProbability::new(mid), &tuning);
    assert_eq!(
        clamped.to_bits(),
        mid.to_bits(),
        "a value already in [p_min, p_max] must pass through unchanged",
    );
}


const ROLL_LEN: usize = 64;

#[test]
fn rolls_interrupt_is_deterministic_replayable_under_same_seed() {
    let tuning = ReactionTuning::default();
    let p = interrupt_probability(ReactionScore::new(3.0), ReactionScore::new(3.0), &tuning);

    let mut a = ReactionRng::from_root(BattleSeed::new(0x0467_C0DE_DEAD_BEEF));
    let mut b = ReactionRng::from_root(BattleSeed::new(0x0467_C0DE_DEAD_BEEF));

    let seq_a: Vec<bool> = (0..ROLL_LEN).map(|_| *rolls_interrupt(p, &mut a)).collect();
    let seq_b: Vec<bool> = (0..ROLL_LEN).map(|_| *rolls_interrupt(p, &mut b)).collect();

    assert_eq!(
        seq_a, seq_b,
        "same seed + same probability must yield an identical interrupt sequence",
    );
}


#[test]
fn higher_watcher_score_yields_strictly_higher_probability() {
    let tuning = ReactionTuning::default();
    let mover = ReactionScore::new(4.0);
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

#[test]
fn mover_spending_more_tu_lowers_its_score_and_raises_watcher_probability() {
    let tuning = ReactionTuning::default();
    let reactions = Reactions::new(4.0);
    let tu_max = TuMax::new(10);

    let mover_full = reaction_score(reactions, Tu::new(8), tu_max);
    let mover_spent = reaction_score(reactions, Tu::new(2), tu_max);
    assert!(
        *mover_spent < *mover_full,
        "spending TU must lower the mover's score: spent={} !< full={}",
        *mover_spent,
        *mover_full,
    );

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


#[test]
fn extremes_clamp_strictly_inside_zero_and_one() {
    let tuning = ReactionTuning::default();

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


#[test]
fn cap_blocks_at_limit_and_reset_reopens() {
    let tuning = ReactionTuning::default();
    let reactions = Reactions::new(4.0);
    let cap = *reaction_cap(reactions, &tuning);

    let used_below = ReactionsUsed::new(cap.saturating_sub(1));
    assert!(
        *may_interrupt(used_below, reactions, &tuning),
        "with used < cap, may_interrupt must be true",
    );

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

    used.reset();
    assert_eq!(*used, 0, "reset must zero the counter");
    assert!(
        *may_interrupt(used, reactions, &tuning),
        "after reset, may_interrupt must be true again",
    );
}


#[test]
fn reaction_score_with_zero_tu_max_is_zero_and_finite() {
    let score = reaction_score(Reactions::new(5.0), Tu::new(3), TuMax::new(0));
    assert!(
        score.is_finite(),
        "reaction_score with tu_max==0 must be finite, got {}",
        *score,
    );
    assert_eq!(
        score.to_bits(),
        0.0_f32.to_bits(),
        "reaction_score with tu_max==0 must be exactly 0.0 (no div-by-zero)",
    );
}

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
