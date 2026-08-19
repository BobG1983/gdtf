use super::support::*;
use crate::resolve_and_apply::{ShotSource, WoundRoll};

#[test]
fn sufficient_hit_destroys_slab_and_records_the_cell() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(60, 40, 20, DamageType::Kinetic);
    let at = slab_cell_level();

    let mut slab = slab_ledger();
    slab.insert(at, slab_entry(20, 2, 1));
    let mut cover = ledger();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &slab_outcome(),
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng_used,
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut injury_rng(),
        },
    );

    assert_eq!(
        report.verdict,
        HitVerdict::Slab(SlabVerdict {
            destroyed: Some(at),
        }),
        "a sufficient slab hit must record the destroyed (cell, level) on the verdict",
    );

    let after = slab.peek(&at).copied();
    assert!(
        after.is_some_and(|e| *e.destroyed && *e.current_hp == 0),
        "a destroyed slab must read destroyed == true at zero HP",
    );

    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a slab hit must take NO RNG draw",
    );
}

#[test]
fn insufficient_hit_reduces_hp_without_destroying() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(12, 6, 2, DamageType::Kinetic);
    let max_hp = 400_u32;
    let at = slab_cell_level();

    let mut slab = slab_ledger();
    slab.insert(at, slab_entry(max_hp, 2, 1));
    let mut cover = ledger();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &slab_outcome(),
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng_used,
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut injury_rng(),
        },
    );

    assert_eq!(
        report.verdict,
        HitVerdict::Slab(SlabVerdict { destroyed: None }),
        "an insufficient slab hit must record NO destroyed cell",
    );

    let after = slab.peek(&at).copied();
    assert!(
        after.is_some_and(|e| !*e.destroyed && *e.current_hp < max_hp && *e.current_hp > 0),
        "an insufficient hit must REDUCE the slab HP below max but leave it above zero",
    );

    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a slab hit must take NO RNG draw",
    );
}

#[test]
fn slab_hit_is_deterministic_under_seeded_rng() {
    let run = || {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(30, 20, 8, DamageType::Kinetic);
        let at = slab_cell_level();
        let mut slab = slab_ledger();
        slab.insert(at, slab_entry(50, 3, 2));
        let mut cover = ledger();
        let mut r = rng();
        let report = resolve_and_apply(
            &slab_outcome(),
            ShotSource {
                weapon: weapon.stats(),
                luck:   Luck::new(0.0),
            },
            None,
            an_entity(),
            surfaces(&mut cover, &mut slab),
            &mut WoundRoll {
                tuning:       &tuning,
                severity_rng: &mut r,
                tables:       &injury_tables(),
                registry:     &injury_registry(),
                injury_rng:   &mut injury_rng(),
            },
        );
        (report, slab.peek(&at).copied())
    };

    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same slab report AND ledger state",
    );
}

#[test]
fn a_fold_at_a_cell_with_no_entry_destroys_nothing_and_mints_nothing() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(60, 40, 20, DamageType::Kinetic);
    let at = slab_cell_level();

    let mut slab = slab_ledger();
    let mut cover = ledger();

    let report = resolve_and_apply(
        &slab_outcome(),
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng(),
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut injury_rng(),
        },
    );

    assert_eq!(
        report.verdict,
        HitVerdict::Slab(SlabVerdict { destroyed: None }),
        "a cell with no authored slab must record NO destroyed cell",
    );
    assert!(
        slab.peek(&at).is_none(),
        "the fold must leave the ledger empty at a cell with no authored slab; found {:?}",
        slab.peek(&at),
    );
}

/// Two slabs alike but for hardness, so only the entry's own armor can tell them apart.
const SLAB_MAX_HP: u32 = 100;
const SLAB_PROTECTION: i32 = 10;
const SOFT_HARDNESS: i32 = 2;
const HARD_HARDNESS: i32 = 6;
const HIT_DAMAGE: i32 = 20;
const HIT_PUNCH: i32 = 8;

fn slab_after_one_hit(hardness: i32, tuning: &CombatTuning) -> Option<SlabEntry> {
    let weapon = a_weapon(HIT_DAMAGE, HIT_PUNCH, 0, DamageType::Kinetic);
    let at = slab_cell_level();

    let mut slab = slab_ledger();
    slab.insert(at, slab_entry(SLAB_MAX_HP, SLAB_PROTECTION, hardness));
    let mut cover = ledger();

    let report = resolve_and_apply(
        &slab_outcome(),
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning,
            severity_rng: &mut rng(),
            tables: &injury_tables(),
            registry: &injury_registry(),
            injury_rng: &mut injury_rng(),
        },
    );
    assert_eq!(
        report.verdict,
        HitVerdict::Slab(SlabVerdict { destroyed: None }),
        "hardness {hardness}: one hit must not destroy the slab, or the two remainders cannot \
         be compared",
    );
    slab.peek(&at).copied()
}

#[test]
fn a_harder_slab_keeps_more_hp_than_a_softer_one_under_the_same_hit() {
    let tuning = CombatTuning::default();

    let (Some(soft), Some(hard)) = (
        slab_after_one_hit(SOFT_HARDNESS, &tuning),
        slab_after_one_hit(HARD_HARDNESS, &tuning),
    ) else {
        unreachable!("both folds insert their entry before firing, so both peeks answer Some")
    };

    assert!(
        *soft.current_hp > 0 && *soft.current_hp < SLAB_MAX_HP,
        "the softer slab must take non-zero, non-lethal damage or the case cannot \
         discriminate; it holds {} of {SLAB_MAX_HP}",
        *soft.current_hp,
    );
    assert!(
        *hard.current_hp > 0 && *hard.current_hp < SLAB_MAX_HP,
        "the harder slab must take non-zero, non-lethal damage or the case cannot \
         discriminate; it holds {} of {SLAB_MAX_HP}",
        *hard.current_hp,
    );
    assert!(
        *hard.current_hp > *soft.current_hp,
        "the fold must resolve against each entry's OWN armor: the harder slab kept {} HP and \
         the softer one {} — equal remainders mean one shared armor piece resolved both",
        *hard.current_hp,
        *soft.current_hp,
    );
}
