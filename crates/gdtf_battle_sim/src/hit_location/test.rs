//! Relocated unit tests for the weighted hit-location roll (GTW-201 wave 22 — moved
//! verbatim from the former inline `#[cfg(test)] mod tests`).

use rand::{SeedableRng, rngs::StdRng};

use crate::{
    armor::BodyPart,
    hit_location::roll_body_part,
    tuning::{BodyPartWeight, BodyPartWeights},
};

/// Build a [`BodyPartWeights`] from six raw per-part weights, in
/// [`BodyPart::ALL`] order. Helper so tests state ARBITRARY weights inline
/// (never the shipped tuning magnitudes) — pinning the roll MECHANISM, not a
/// balance number.
fn weights(
    head: u16,
    torso: u16,
    l_arm: u16,
    r_arm: u16,
    l_leg: u16,
    r_leg: u16,
) -> BodyPartWeights {
    BodyPartWeights {
        head:      BodyPartWeight::new(head),
        torso:     BodyPartWeight::new(torso),
        left_arm:  BodyPartWeight::new(l_arm),
        right_arm: BodyPartWeight::new(r_arm),
        left_leg:  BodyPartWeight::new(l_leg),
        right_leg: BodyPartWeight::new(r_leg),
    }
}

/// AC #1: a seeded RNG yields a deterministic pick on the REAL path. Seeding
/// the stream and calling the real `roll_body_part` returns one fixed part,
/// and re-running from the same seed returns the same part — the roll is a
/// pure function of (weights, RNG state).
#[test]
fn seeded_roll_is_a_deterministic_pick_on_the_real_path() {
    let w = weights(6, 40, 15, 15, 12, 12);
    let mut a = StdRng::seed_from_u64(0xA11CE);
    let mut b = StdRng::seed_from_u64(0xA11CE);
    let first = roll_body_part(&w, &mut a);
    let again = roll_body_part(&w, &mut b);
    assert_eq!(
        first, again,
        "same seed + same weights must give the same first pick",
    );
}

/// AC #4: same seed → same SEQUENCE of parts (determinism), every draw via the
/// injected RNG. Two identically-seeded streams produce the same run of picks
/// across many rolls — value-for-value.
#[test]
fn same_seed_same_sequence_of_parts() {
    let w = weights(6, 40, 15, 15, 12, 12);
    let mut a = StdRng::seed_from_u64(0xBEEF);
    let mut b = StdRng::seed_from_u64(0xBEEF);
    let seq_a: Vec<BodyPart> = (0..256).map(|_| roll_body_part(&w, &mut a)).collect();
    let seq_b: Vec<BodyPart> = (0..256).map(|_| roll_body_part(&w, &mut b)).collect();
    assert_eq!(seq_a, seq_b, "same seed must replay the same part sequence");
}

/// AC #2: over a large seeded sample the empirical frequencies track the
/// configured weights — asserting ORDERING/proportion, NEVER exact counts.
/// With the default-shaped weights (torso heaviest, head lightest) the sample
/// must rank torso markedly above head, torso above each limb, and each limb
/// above head — a tuning edit that keeps that ordering keeps this green.
#[test]
fn distribution_tracks_weight_ordering_not_exact_counts() {
    // Default-SHAPED weights (head rare, torso the bulk) — used for the
    // ordering relation only, not pinned as magnitudes.
    let w = weights(6, 40, 15, 15, 12, 12);
    let mut rng = StdRng::seed_from_u64(0x5EED_0166);

    let mut counts = [0_u32; 6];
    let samples = 60_000;
    for _ in 0..samples {
        counts[roll_body_part(&w, &mut rng).index()] += 1;
    }

    let head = counts[BodyPart::Head.index()];
    let torso = counts[BodyPart::Torso.index()];
    let l_arm = counts[BodyPart::LeftArm.index()];
    let r_arm = counts[BodyPart::RightArm.index()];
    let l_leg = counts[BodyPart::LeftLeg.index()];
    let r_leg = counts[BodyPart::RightLeg.index()];

    // Torso (weight 40) markedly more frequent than head (weight 6): a wide
    // margin, not a tight count — torso should land several times as often.
    assert!(
        torso > head * 3,
        "torso (heaviest) must be markedly more frequent than head (lightest): \
         torso={torso} head={head}",
    );
    // Torso outranks every limb; every limb outranks head — the configured
    // ordering, holding with comfortable slack so it is not count-brittle.
    assert!(
        torso > l_arm && torso > r_arm,
        "torso must outrank the arms"
    );
    assert!(
        torso > l_leg && torso > r_leg,
        "torso must outrank the legs"
    );
    assert!(l_arm > head && r_arm > head, "each arm must outrank head");
    assert!(l_leg > head && r_leg > head, "each leg must outrank head");
}

/// AC #3 (zero-weight never picked): a zero-weight part is NEVER selected over
/// a degenerate weights set built from ARBITRARY weights. Head is zeroed; over
/// a large sample it must never come up, while the non-zero parts all do.
#[test]
fn zero_weight_part_is_never_picked() {
    // Arbitrary weights — head zeroed, the rest non-zero (not shipped values).
    let w = weights(0, 7, 3, 3, 5, 5);
    let mut rng = StdRng::seed_from_u64(0x0FF0);

    let mut head_seen = false;
    let mut non_head_seen = false;
    for _ in 0..20_000 {
        match roll_body_part(&w, &mut rng) {
            BodyPart::Head => head_seen = true,
            _ => non_head_seen = true,
        }
    }
    assert!(!head_seen, "a zero-weight part must NEVER be picked");
    assert!(non_head_seen, "the non-zero parts must still be picked");
}

/// AC #3 (single non-zero always picked): when exactly one part is non-zero it
/// is ALWAYS the result, regardless of RNG draw. Arbitrary single-weight set;
/// every roll over many draws returns that one part.
#[test]
fn single_non_zero_part_is_always_picked() {
    // Only the left leg carries weight — arbitrary magnitude.
    let w = weights(0, 0, 0, 0, 9, 0);
    let mut rng = StdRng::seed_from_u64(0xCAFE);
    for _ in 0..5_000 {
        assert_eq!(
            roll_body_part(&w, &mut rng),
            BodyPart::LeftLeg,
            "the sole non-zero part must always be picked",
        );
    }
}

/// AC #5: an all-six-zero weights total is handled gracefully — the documented
/// [`BodyPart::Torso`] fallback, no panic, and without consuming the RNG. The
/// fallback never draws, so a subsequent roll over real weights starts from the
/// RNG's pristine first draw.
#[test]
fn all_zero_weights_fall_back_to_torso_without_panic() {
    let zero = weights(0, 0, 0, 0, 0, 0);
    let mut rng = StdRng::seed_from_u64(0x0D15_EA5E);
    // Many calls all return the deterministic central-mass fallback, no panic.
    for _ in 0..1_000 {
        assert_eq!(
            roll_body_part(&zero, &mut rng),
            BodyPart::Torso,
            "all-zero weights must fall back to torso, never panic",
        );
    }

    // The fallback did NOT touch the RNG: a fresh stream and a stream that was
    // first hammered with all-zero rolls produce the SAME first real pick.
    let real = weights(6, 40, 15, 15, 12, 12);
    let mut untouched = StdRng::seed_from_u64(0x1357);
    let mut after_zeros = StdRng::seed_from_u64(0x1357);
    for _ in 0..50 {
        let _drained = roll_body_part(&zero, &mut after_zeros);
    }
    assert_eq!(
        roll_body_part(&real, &mut untouched),
        roll_body_part(&real, &mut after_zeros),
        "the all-zero fallback must not consume the RNG stream",
    );
}
