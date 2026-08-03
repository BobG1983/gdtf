use super::support::*;

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
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
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
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
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
            weapon.stats(),
            Luck::new(0.0),
            None,
            an_entity(),
            surfaces(&mut cover, &mut slab),
            &tuning,
            &mut r,
            &injury_tables(),
            &injury_registry(),
            &mut injury_rng(),
        );
        (report, slab.peek(&at).copied())
    };

    assert_eq!(
        run(),
        run(),
        "the same seed + inputs must reproduce the same slab report AND ledger state",
    );
}
