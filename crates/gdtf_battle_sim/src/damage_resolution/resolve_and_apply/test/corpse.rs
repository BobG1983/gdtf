use super::support::*;

/// AC2 — corpse-skip: a `Ganger` outcome on an already-`Dead` target yields a
/// no-effect report and mutates nothing; AND it draws nothing — proven by the
/// `SeverityRng` stream being unchanged vs a fresh one after the call.
#[test]
fn corpse_skip_is_inert_and_draws_nothing() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Head;
    let weapon = a_weapon(20, 12, 8, DamageType::Plasma);
    let outcome = ganger_outcome(entity, part);

    let mut hp = Hp::new(15);
    let mut wounds = Wounds::new(3);
    let mut life = LifeState::Dead; // already a corpse
    let mut integrity = piece_integrity(1); // a live hit WOULD break it
    let mut inflicted = InflictedWounds::default();

    let hp_before = hp;
    let wounds_before = wounds;
    let life_before = life;
    let integrity_before = integrity;

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(5.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     Some(struck_piece(0, 0, 0, ArmorType::DEFAULT, &mut integrity)),
            inflicted: &mut inflicted,
            toughness: Toughness::new(2.0),
            luck:      Luck::new(1.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut rng_used,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );

    // No-effect report.
    assert_eq!(report.applied, None, "a corpse-skip must apply no damage");
    assert_eq!(report.part, None, "a corpse-skip report carries no part");
    assert_eq!(
        report.kind,
        ShotKind::Ganger(entity),
        "the report still names what the shot struck",
    );

    // Nothing mutated.
    assert_eq!(hp, hp_before, "a corpse's Hp must not change");
    assert_eq!(wounds, wounds_before, "a corpse's Wounds must not change");
    assert_eq!(life, life_before, "a corpse's LifeState must stay Dead");
    assert_eq!(
        integrity, integrity_before,
        "a corpse's struck-piece integrity must not wear"
    );
    assert!(
        inflicted.is_empty(),
        "a corpse-skip must record NO InflictedWound (GTW-279)",
    );

    // No draw was taken: the used RNG's next draw matches a fresh stream's
    // first draw (the cursor never advanced).
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "corpse-skip must take NO draw — the SeverityRng cursor must be unadvanced",
    );
}
