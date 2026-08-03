use super::fight::{
    FightMargin, FightOutcome, MeleeDamageMult, apply_melee_multiplier, melee_damage_mult,
    opposed_fight,
};
use crate::{
    ganger::Fight,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    rng::{BattleSeed, FightRng, ShotRng},
    tuning::MeleeTuning,
};

const SAMPLE_LEN: usize = 4096;

const SEED: u64 = 0x0506_C0DE_FACE_B00C;


#[test]
fn opposed_fight_is_deterministic_replayable_under_same_seed() {
    let tuning = MeleeTuning::default();
    let attacker = Fight::new(5.0);
    let defender = Fight::new(4.0);

    let mut a = FightRng::from_root(BattleSeed::new(SEED));
    let mut b = FightRng::from_root(BattleSeed::new(SEED));

    let seq_a: Vec<FightOutcome> = (0..256)
        .map(|_| opposed_fight(attacker, defender, tuning.variance, &mut a))
        .collect();
    let seq_b: Vec<FightOutcome> = (0..256)
        .map(|_| opposed_fight(attacker, defender, tuning.variance, &mut b))
        .collect();

    assert_eq!(
        seq_a, seq_b,
        "same seed + same inputs must yield an identical FightOutcome sequence",
    );
}


fn connect_rate_and_mean_margin(attacker: f32, defender: f32) -> (usize, f32) {
    let tuning = MeleeTuning::default();
    let mut rng = FightRng::from_root(BattleSeed::new(SEED));
    let mut connects = 0usize;
    let mut margin_sum = 0.0_f32;
    for _ in 0..SAMPLE_LEN {
        let outcome = opposed_fight(
            Fight::new(attacker),
            Fight::new(defender),
            tuning.variance,
            &mut rng,
        );
        if *outcome.connect {
            connects += 1;
        }
        margin_sum += *outcome.margin;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "SAMPLE_LEN is far inside f32's exact-integer range; the mean is a test statistic, not a pinned magnitude"
    )]
    let mean = margin_sum / SAMPLE_LEN as f32;
    (connects, mean)
}

#[test]
fn higher_attacker_fight_raises_connect_rate_and_mean_margin() {
    let defender = 4.0;
    let (connects_weak, mean_weak) = connect_rate_and_mean_margin(3.0, defender);
    let (connects_strong, mean_strong) = connect_rate_and_mean_margin(6.0, defender);

    assert!(
        connects_strong > connects_weak,
        "a higher attacker Fight must connect MORE often: strong={connects_strong} !> weak={connects_weak} \
         (FAILS if atk/def is inverted)",
    );
    assert!(
        mean_strong > mean_weak,
        "a higher attacker Fight must produce a higher mean margin: strong={mean_strong} !> weak={mean_weak} \
         (FAILS if atk/def is inverted)",
    );
}


#[test]
fn doubling_both_fights_leaves_margin_distribution_unchanged() {
    let tuning = MeleeTuning::default();
    let mut base = FightRng::from_root(BattleSeed::new(SEED));
    let mut scaled = FightRng::from_root(BattleSeed::new(SEED));

    for i in 0..SAMPLE_LEN {
        let m_base =
            *opposed_fight(Fight::new(3.0), Fight::new(5.0), tuning.variance, &mut base).margin;
        let m_scaled = *opposed_fight(
            Fight::new(6.0),
            Fight::new(10.0),
            tuning.variance,
            &mut scaled,
        )
        .margin;
        assert_eq!(
            m_base.to_bits(),
            m_scaled.to_bits(),
            "margin is relative (atk/def − 1): doubling both Fights must not change it \
             (sample {i}: base={m_base}, scaled={m_scaled})",
        );
    }
}


#[test]
fn melee_damage_mult_is_clamped_monotone_and_glancing_reachable() {
    let tuning = MeleeTuning::default();

    let at_floor = melee_damage_mult(FightMargin::new(-1.0e6), &tuning);
    assert_eq!(
        at_floor.to_bits(),
        (*tuning.mult_min).to_bits(),
        "a hugely negative margin must clamp to mult_min",
    );
    let at_ceiling = melee_damage_mult(FightMargin::new(1.0e6), &tuning);
    assert_eq!(
        at_ceiling.to_bits(),
        (*tuning.mult_max).to_bits(),
        "a hugely positive margin must clamp to mult_max",
    );

    let margins = [-2.0_f32, -0.5, -0.1, 0.0, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0];
    let mults: Vec<f32> = margins
        .iter()
        .map(|&m| *melee_damage_mult(FightMargin::new(m), &tuning))
        .collect();
    for window in mults.windows(2) {
        let (prev, next) = (window[0], window[1]);
        assert!(
            prev <= next,
            "melee_damage_mult must be monotone non-decreasing in margin: {prev} > {next}",
        );
    }

    let at_zero = melee_damage_mult(FightMargin::new(0.0), &tuning);
    assert_eq!(
        at_zero.to_bits(),
        (*tuning.mult_min).to_bits(),
        "at margin 0 the multiplier must equal mult_min (the additive base)",
    );
    assert!(
        *at_zero < 1.0,
        "mult_min < 1 must be reachable so a barely-connecting hit GLANCES (got {})",
        *at_zero,
    );
}


#[test]
fn degenerate_zero_defender_fight_is_defined_and_finite() {
    let tuning = MeleeTuning::default();
    let mut rng = FightRng::from_root(BattleSeed::new(SEED));

    for i in 0..64 {
        let outcome = opposed_fight(Fight::new(5.0), Fight::new(0.0), tuning.variance, &mut rng);
        assert!(
            outcome.margin.is_finite(),
            "degenerate def<=0 margin must be finite (sample {i}), got {}",
            *outcome.margin,
        );
        assert!(
            *outcome.connect,
            "degenerate def<=0 must connect (a defenceless target is hit), sample {i}",
        );
        let mult = melee_damage_mult(outcome.margin, &tuning);
        assert!(
            mult.is_finite(),
            "degenerate multiplier must be finite, got {}",
            *mult
        );
        assert_eq!(
            mult.to_bits(),
            (*tuning.mult_max).to_bits(),
            "degenerate def<=0 must saturate the multiplier to mult_max (sample {i})",
        );
    }
}


#[test]
fn apply_melee_multiplier_scales_damage_by_the_factor() {
    let hit = HitResult {
        penetrating: PenetratingDamage::new(7),
        hp_damage:   HpDamage::new(10),
        wear:        IntegrityWear::new(4),
    };

    let doubled = apply_melee_multiplier(hit, MeleeDamageMult::new(2.0));
    assert_eq!(*doubled.hp_damage, 20, "a 2x mult must double hp_damage");
    assert_eq!(
        *doubled.penetrating, 14,
        "a 2x mult must double penetrating"
    );
    assert_eq!(*doubled.wear, 8, "a 2x mult must double integrity wear");

    let identity = apply_melee_multiplier(hit, MeleeDamageMult::new(1.0));
    assert_eq!(
        identity, hit,
        "a 1x mult must leave the HitResult unchanged"
    );

    let glancing = apply_melee_multiplier(hit, MeleeDamageMult::new(0.5));
    assert_eq!(*glancing.hp_damage, 5, "a 0.5x mult must halve hp_damage");
    assert_eq!(
        *glancing.penetrating, 4,
        "a 0.5x mult rounds 7*0.5=3.5 to 4 (half away from zero)"
    );
    assert_eq!(*glancing.wear, 2, "a 0.5x mult must halve integrity wear");
}


const STREAM_LEN: usize = 64;

#[test]
fn fight_rng_is_deterministic_under_same_seed() {
    let root = BattleSeed::new(SEED);
    let a: Vec<u64> = {
        let mut r = FightRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let b: Vec<u64> = {
        let mut r = FightRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    assert_eq!(
        a, b,
        "two FightRngs from the same root must draw the identical stream"
    );
}

#[test]
fn fight_rng_does_not_perturb_existing_shot_stream() {
    let root = BattleSeed::new(0xC0_FFEE_5060);

    let baseline: Vec<u64> = {
        let mut r = ShotRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };

    let mut fight = FightRng::from_root(root);
    for _ in 0..STREAM_LEN {
        fight.next_u64();
    }
    let after_fight_draws: Vec<u64> = {
        let mut r = ShotRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };

    assert_eq!(
        baseline, after_fight_draws,
        "ShotRng's output must be unaffected by prior FightRng draws — the streams are \
         independent (different labels → different FNV hash → different ChaCha12 key)",
    );

    let fight_seq: Vec<u64> = {
        let mut r = FightRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    assert_ne!(
        baseline, fight_seq,
        "FightRng and ShotRng must produce different sequences from the same root",
    );
}
