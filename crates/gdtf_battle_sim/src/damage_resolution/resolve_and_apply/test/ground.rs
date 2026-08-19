use super::support::*;
use crate::resolve_and_apply::{ShotSource, WoundRoll};

#[test]
fn ground_hit_accrues_weapon_damage_on_the_correct_cell() {
    let tuning = CombatTuning::default();
    let weapon_damage = 37_i32;
    let weapon = a_weapon(weapon_damage, 10, 4, DamageType::Kinetic);
    let at = ground_cell_level();
    let expected_cell = at.cell();

    let mut cover = ledger();
    let mut slab = slab_ledger();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &ground_outcome(),
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
        HitVerdict::Ground(GroundAccrual::new(
            expected_cell,
            GroundDamage::new(u32::try_from(weapon_damage).unwrap_or(0)),
        )),
        "a ground hit must record the round's weapon_damage against the struck cell",
    );
}

#[test]
fn ground_hit_touches_no_ganger_cover_or_slab_state() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(50, 30, 12, DamageType::Kinetic);

    let at = ground_cell_level();
    let mut cover = ledger();
    cover.insert(at, cover_entry(100, 2, 1, TerrainPieceKind::Cover));
    let cover_before = cover.peek(&at).copied();
    let mut slab = slab_ledger();
    slab.insert(at, slab_entry(100, 2, 1));
    let slab_before = slab.peek(&at).copied();

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &ground_outcome(),
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

    assert!(
        matches!(report.verdict, HitVerdict::Ground(_)),
        "a ground hit must fold to a Ground accrual verdict, got {:?}",
        report.verdict,
    );

    assert_eq!(
        cover.peek(&at).copied(),
        cover_before,
        "a ground hit must not deplete the cover ledger (C3 — only the ground accrues)",
    );
    assert_eq!(
        slab.peek(&at).copied(),
        slab_before,
        "a ground hit must not deplete the slab ledger (C3 — only the ground accrues)",
    );
}

#[test]
fn ground_hit_takes_no_rng_draw_and_is_deterministic() {
    let run = || {
        let tuning = CombatTuning::default();
        let weapon = a_weapon(29, 14, 6, DamageType::Kinetic);
        let mut cover = ledger();
        let mut slab = slab_ledger();
        let mut r = rng();
        let report = resolve_and_apply(
            &ground_outcome(),
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
        report.verdict
    };

    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same ground accrual",
    );

    let tuning = CombatTuning::default();
    let weapon = a_weapon(29, 14, 6, DamageType::Kinetic);
    let mut cover = ledger();
    let mut slab = slab_ledger();
    let mut rng_used = rng();
    let _report = resolve_and_apply(
        &ground_outcome(),
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
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "a ground hit must take NO RNG draw (replay-deterministic)",
    );
}
