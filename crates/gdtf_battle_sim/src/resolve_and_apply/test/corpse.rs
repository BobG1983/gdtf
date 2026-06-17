use super::support::*;

/// AC2 — corpse-skip: a `Ganger` outcome on an already-`Dead` target yields a
/// no-effect report and mutates nothing; AND it draws nothing — proven by the
/// `SimRng` stream being unchanged vs a fresh one after the call.
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
    let mut worn = worn_suit(0, 0, 1, 0, ArmorType::DEFAULT); // a live hit WOULD break it

    let hp_before = hp;
    let wounds_before = wounds;
    let life_before = life;
    let worn_before = worn;

    let mut rng_used = rng();
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(5.0),
        TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            worn:      &mut worn,
            toughness: Toughness::new(2.0),
            luck:      Luck::new(1.0),
        },
        entity,
        &tuning,
        &mut rng_used,
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
    assert_eq!(worn, worn_before, "a corpse's WornArmor must not wear");

    // No draw was taken: the used RNG's next draw matches a fresh stream's
    // first draw (the cursor never advanced).
    let mut rng_fresh = rng();
    assert_eq!(
        rng_used.next_u64(),
        rng_fresh.next_u64(),
        "corpse-skip must take NO draw — the SimRng cursor must be unadvanced",
    );
}
