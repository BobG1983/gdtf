use super::support::*;

#[test]
fn corpse_skip_is_inert_and_draws_nothing() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Head;
    let weapon = a_weapon(20, 12, 8, DamageType::Plasma);
    let outcome = ganger_outcome(entity, part);

    let mut hp = Hp::new(15);
    let mut wounds = Wounds::new(3);
    let mut life = LifeState::Dead; 
    let mut integrity = piece_integrity(1); 
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

    assert_eq!(
        report.verdict,
        HitVerdict::NoEffect,
        "a corpse-skip must fold to a no-effect verdict",
    );
    assert_eq!(
        report.kind,
        ShotKind::Ganger(entity),
        "the report still names what the shot struck",
    );

    assert_eq!(hp, hp_before, "a corpse's Hp must not change");
    assert_eq!(wounds, wounds_before, "a corpse's Wounds must not change");
    assert_eq!(life, life_before, "a corpse's LifeState must stay Dead");
    assert_eq!(
        integrity, integrity_before,
        "a corpse's struck-piece integrity must not wear"
    );
    assert!(
        inflicted.is_empty(),
        "a corpse-skip must record NO InflictedWound ",
    );

    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "corpse-skip must take NO draw — the SeverityRng cursor must be unadvanced",
    );
}
