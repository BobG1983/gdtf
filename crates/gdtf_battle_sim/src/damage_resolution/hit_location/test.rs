//! verbatim from the former inline `#[cfg(test)] mod tests`).

use crate::{
    armor::BodyPart,
    hit_location::roll_body_part,
    rng::{BattleSeed, ShotRng},
    tuning::{BodyPartWeight, BodyPartWeights},
};

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

#[test]
fn seeded_roll_is_a_deterministic_pick_on_the_real_path() {
    let w = weights(6, 40, 15, 15, 12, 12);
    let mut a = ShotRng::from_root(BattleSeed::new(0xA11CE));
    let mut b = ShotRng::from_root(BattleSeed::new(0xA11CE));
    let first = roll_body_part(&w, a.rng());
    let again = roll_body_part(&w, b.rng());
    assert_eq!(
        first, again,
        "same seed + same weights must give the same first pick",
    );
}

#[test]
fn same_seed_same_sequence_of_parts() {
    let w = weights(6, 40, 15, 15, 12, 12);
    let mut a = ShotRng::from_root(BattleSeed::new(0xBEEF));
    let mut b = ShotRng::from_root(BattleSeed::new(0xBEEF));
    let seq_a: Vec<BodyPart> = (0..256).map(|_| roll_body_part(&w, a.rng())).collect();
    let seq_b: Vec<BodyPart> = (0..256).map(|_| roll_body_part(&w, b.rng())).collect();
    assert_eq!(seq_a, seq_b, "same seed must replay the same part sequence");
}

#[test]
fn distribution_tracks_weight_ordering_not_exact_counts() {
    let w = weights(6, 40, 15, 15, 12, 12);
    let mut shot = ShotRng::from_root(BattleSeed::new(0x5EED_0166));

    let mut counts = [0_u32; 6];
    let samples = 60_000;
    for _ in 0..samples {
        counts[roll_body_part(&w, shot.rng()).index()] += 1;
    }

    let head = counts[BodyPart::Head.index()];
    let torso = counts[BodyPart::Torso.index()];
    let l_arm = counts[BodyPart::LeftArm.index()];
    let r_arm = counts[BodyPart::RightArm.index()];
    let l_leg = counts[BodyPart::LeftLeg.index()];
    let r_leg = counts[BodyPart::RightLeg.index()];

    assert!(
        torso > head * 3,
        "torso (heaviest) must be markedly more frequent than head (lightest): \
         torso={torso} head={head}",
    );
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

#[test]
fn zero_weight_part_is_never_picked() {
    let w = weights(0, 7, 3, 3, 5, 5);
    let mut shot = ShotRng::from_root(BattleSeed::new(0x0FF0));

    let mut head_seen = false;
    let mut non_head_seen = false;
    for _ in 0..20_000 {
        match roll_body_part(&w, shot.rng()) {
            BodyPart::Head => head_seen = true,
            _ => non_head_seen = true,
        }
    }
    assert!(!head_seen, "a zero-weight part must NEVER be picked");
    assert!(non_head_seen, "the non-zero parts must still be picked");
}

#[test]
fn single_non_zero_part_is_always_picked() {
    let w = weights(0, 0, 0, 0, 9, 0);
    let mut shot = ShotRng::from_root(BattleSeed::new(0xCAFE));
    for _ in 0..5_000 {
        assert_eq!(
            roll_body_part(&w, shot.rng()),
            BodyPart::LeftLeg,
            "the sole non-zero part must always be picked",
        );
    }
}

#[test]
fn all_zero_weights_fall_back_to_torso_without_panic() {
    let zero = weights(0, 0, 0, 0, 0, 0);
    let mut shot = ShotRng::from_root(BattleSeed::new(0x0D15_EA5E));
    for _ in 0..1_000 {
        assert_eq!(
            roll_body_part(&zero, shot.rng()),
            BodyPart::Torso,
            "all-zero weights must fall back to torso, never panic",
        );
    }

    let real = weights(6, 40, 15, 15, 12, 12);
    let mut untouched = ShotRng::from_root(BattleSeed::new(0x1357));
    let mut after_zeros = ShotRng::from_root(BattleSeed::new(0x1357));
    for _ in 0..50 {
        let _drained = roll_body_part(&zero, after_zeros.rng());
    }
    assert_eq!(
        roll_body_part(&real, untouched.rng()),
        roll_body_part(&real, after_zeros.rng()),
        "the all-zero fallback must not consume the RNG stream",
    );
}
