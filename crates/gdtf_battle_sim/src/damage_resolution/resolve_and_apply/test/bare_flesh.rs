use super::support::*;

#[test]
fn bare_flesh_uses_no_protection_or_hardness() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::RightArm;
    let weapon = a_weapon(10, 3, 0, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut integrity = piece_integrity(0);
    let mut inflicted = InflictedWounds::default();
    assert!(
        *integrity <= 0,
        "fixture: the struck piece must already be worn through (no protection)",
    );

    let report = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     Some(struck_piece(2, 20, 5, ArmorType::Void, &mut integrity)),
            inflicted: &mut inflicted,
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );

    assert!(
        applied_of(&report).is_some(),
        "a ganger hit must carry an applied-damage block",
    );
    let Some(applied) = applied_of(&report) else {
        return;
    };
    assert_eq!(
        applied.matchup,
        Matchup::Neutral,
        "bare flesh has no armor type to match → Neutral (no wheel advantage)",
    );
    assert_eq!(
        *applied.hit.hp_damage, *weapon.damage,
        "bare flesh deals full weapon damage (no protection / hardness)",
    );
    assert_eq!(
        applied.wear,
        ArmorWearOutcome::Unaffected,
        "bare flesh wears no piece — the wear outcome must be Unaffected",
    );

    let mut hp2 = Hp::new(50);
    let mut wounds2 = Wounds::new(9);
    let mut life2 = LifeState::Alive;
    let mut integrity2 = piece_integrity(50);
    let mut inflicted2 = InflictedWounds::default();
    assert!(
        *integrity2 > 0,
        "fixture: the armored piece must still protect",
    );
    let report2 = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp2,
            wounds:    &mut wounds2,
            life:      &mut life2,
            piece:     Some(struck_piece(2, 20, 5, ArmorType::Void, &mut integrity2)),
            inflicted: &mut inflicted2,
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );
    assert!(
        applied_of(&report2).is_some(),
        "the armored hit must carry an applied-damage block",
    );
    let Some(applied2) = applied_of(&report2) else {
        return;
    };
    assert!(
        *applied2.hit.hp_damage < *applied.hit.hp_damage,
        "the armored hit must take less HP-loss than bare flesh: {} >= {}",
        *applied2.hit.hp_damage,
        *applied.hit.hp_damage,
    );
}
