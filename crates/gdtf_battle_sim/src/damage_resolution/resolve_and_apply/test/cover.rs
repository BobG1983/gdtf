use super::support::*;
use crate::resolve_and_apply::{ShotSource, WoundRoll};

#[test]
fn sufficient_hit_destroys_cover_and_records_the_cell() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(60, 40, 20, DamageType::Kinetic);
    let entry = cover_entry(20, 2, 1);
    let at = cover_cell_level();

    let mut cover = ledger();
    cover.insert(at, entry);

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &cover_outcome(entry),
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab_ledger()),
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
        HitVerdict::Cover(CoverVerdict {
            destroyed: Some(at),
        }),
        "a sufficient cover hit must record the destroyed (cell, level) on the verdict",
    );

    let after = cover.peek(&at).copied();
    assert!(
        after.is_some(),
        "the struck cover entry must still be in the ledger after the hit",
    );
    let Some(after) = after else { return };
    assert!(
        *after.destroyed,
        "a destroyed piece must read destroyed == true"
    );
    assert_eq!(
        *after.current_hp, 0,
        "destruction empties the HP pool to zero"
    );

    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a cover hit must take NO RNG draw",
    );
}

#[test]
fn insufficient_hit_reduces_hp_without_destroying() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(12, 6, 2, DamageType::Kinetic);
    let max_hp = 200_u32;
    let entry = cover_entry(max_hp, 2, 1);
    let at = cover_cell_level();

    let mut cover = ledger();
    cover.insert(at, entry);

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &cover_outcome(entry),
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab_ledger()),
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
        HitVerdict::Cover(CoverVerdict { destroyed: None }),
        "an insufficient cover hit must record NO destroyed cell",
    );

    let after = cover.peek(&at).copied();
    assert!(
        after.is_some(),
        "the struck cover entry must still be in the ledger after the hit",
    );
    let Some(after) = after else { return };
    assert!(
        !*after.destroyed,
        "an insufficient hit must leave the piece NOT destroyed",
    );
    assert!(
        *after.current_hp < max_hp,
        "an insufficient hit must REDUCE the cover HP below its max",
    );
    assert!(
        *after.current_hp > 0,
        "an insufficient hit must leave the cover HP above zero (not destroyed)",
    );

    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a cover hit must take NO RNG draw",
    );
}

#[test]
fn cover_hit_is_deterministic_under_seeded_rng() {
    let run = || {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(30, 20, 8, DamageType::Kinetic);
        let entry = cover_entry(50, 3, 2);
        let at = cover_cell_level();
        let mut cover = ledger();
        cover.insert(at, entry);
        let mut r = rng();
        let report = resolve_and_apply(
            &cover_outcome(entry),
            ShotSource {
                weapon: weapon.stats(),
                luck:   Luck::new(0.0),
            },
            None,
            an_entity(),
            surfaces(&mut cover, &mut slab_ledger()),
            &mut WoundRoll {
                tuning:       &tuning,
                severity_rng: &mut r,
                tables:       &injury_tables(),
                registry:     &injury_registry(),
                injury_rng:   &mut injury_rng(),
            },
        );
        (report, cover.peek(&at).copied())
    };

    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same cover report AND ledger state",
    );
}
