//! The §4 **weighted hit-location roll** — `roll_body_part`.
//!
//! When the coarse march stops a round on a ganger (`docs/combat/resolution.md`
//! §2), this is the entire "where on them" decision: a **weighted pick** over the
//! six [`BodyPart`]s (Head · Torso · L-Arm · R-Arm · L-Leg · R-Leg) by the
//! existing tuning [`BodyPartWeights`] (resolution.md §4 + "What's pure math vs
//! sim" line 152: `roll_body_part(body_part_weights, rng)`). There is **no
//! per-part geometry**: the coarse model already decided *which* ganger by the
//! march's band clearance, and *where* on them is purely this chance roll — the
//! prior on-silhouette / exposure-area model is **retired** (resolution.md §4
//! "RETIRED — both prior models").
//!
//! Every draw bottoms out in the injected [`crate::rng::SimRng`] via the
//! `&mut impl rand::Rng` handle (`docs/testing.md`: "anything random takes an RNG
//! by parameter … never a global/thread RNG"), so the roll is deterministic and
//! seed-replayable. The roll reads **only** the passed [`BodyPartWeights`] — no
//! weight is hardcoded here; the magnitudes (and even the per-part order of their
//! defaults) are tunable balance data (resolution.md §"Coefficients live in the
//! combat-tuning data"). The struck part is the §4 **location** input the E3
//! Injury table and the severity part-mod consume.

use rand::{Rng, RngExt};

use crate::{armor::BodyPart, tuning::BodyPartWeights};

/// Pick the struck [`BodyPart`] by a **weighted roll** over the six parts, using
/// the passed tuning [`BodyPartWeights`] (resolution.md §4).
///
/// The six parts are walked in [`BodyPart::ALL`] canonical order, each weighted
/// by its field in `weights`. A single draw in `0..total` (total = the sum of all
/// six weights) is mapped onto the parts by cumulative weight: the part whose
/// running cumulative weight first exceeds the draw is the one struck. A part with
/// weight `0` owns an empty slice of the range and so can **never** be picked; if
/// exactly one part is non-zero it owns the whole range and is **always** picked
/// (acceptance criterion 3).
///
/// The draw is taken from the injected `rng` (`&mut impl rand::Rng`) — never a
/// global or thread RNG (acceptance criterion 4) — so the same seed yields the
/// same sequence of parts.
///
/// **Degenerate fallback (acceptance criterion 5):** if *all six* weights are
/// zero the total is zero, there is no proportional answer, and there is nothing
/// to draw — so this returns [`BodyPart::Torso`] (the central mass, the natural
/// default landing spot) deterministically, **without panicking** and without
/// touching the RNG. A real tuning never ships all-zero; this is purely a
/// can't-happen guard kept honest.
#[must_use]
pub fn roll_body_part(weights: &BodyPartWeights, rng: &mut impl Rng) -> BodyPart {
    // Sum the six weights as u32 so six u16 weights cannot overflow the total.
    let per_part: [(BodyPart, u32); 6] = [
        (BodyPart::Head, u32::from(*weights.head)),
        (BodyPart::Torso, u32::from(*weights.torso)),
        (BodyPart::LeftArm, u32::from(*weights.left_arm)),
        (BodyPart::RightArm, u32::from(*weights.right_arm)),
        (BodyPart::LeftLeg, u32::from(*weights.left_leg)),
        (BodyPart::RightLeg, u32::from(*weights.right_leg)),
    ];
    let total: u32 = per_part.iter().map(|&(_, w)| w).sum();

    // All-zero fallback: no proportional answer exists, so pick the central mass
    // deterministically rather than draw from an empty range (documented guard).
    let Some(upper) = total.checked_sub(1) else {
        return BodyPart::Torso;
    };

    // One draw in 0..total, mapped onto the parts by cumulative weight. A
    // zero-weight part adds nothing to the running cumulative, so its half-open
    // slice [cumulative, cumulative) is empty and it is never selected.
    let draw: u32 = rng.random_range(0..=upper);
    let mut cumulative: u32 = 0;
    for (part, weight) in per_part {
        cumulative += weight;
        if draw < cumulative {
            return part;
        }
    }

    // Unreachable in practice: `draw < total` and the cumulative sum reaches
    // `total`, so the loop always returns above. Torso is the same central-mass
    // default the all-zero branch uses — no panic, no unwrap.
    BodyPart::Torso
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::tuning::{BodyPartWeight, BodyPartWeights};

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
}
