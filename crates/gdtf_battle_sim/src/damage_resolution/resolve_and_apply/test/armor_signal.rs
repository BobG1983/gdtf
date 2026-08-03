use super::support::*;

#[test]
fn wearing_hit_surfaces_armor_damaged_with_the_delta_and_no_break() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    let weapon = a_weapon(10, 4, 0, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut integrity = piece_integrity(100);
    let mut inflicted = InflictedWounds::default();
    assert!(
        *integrity > 0,
        "fixture: the struck piece must start protecting",
    );

    let report = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     Some(struck_piece(2, 8, 1, ArmorType::Void, &mut integrity)),
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
    assert!(
        *integrity > 0,
        "fixture: a non-breaking hit must leave the piece still protecting",
    );
    assert_eq!(
        applied.wear,
        ArmorWearOutcome::Damaged(ArmorDamaged::new(entity, part, applied.hit.wear)),
        "a wearing-not-breaking hit must surface Damaged carrying the hit's wear delta",
    );
    assert!(
        *applied.hit.wear > 0,
        "fixture: the hit must actually have worn the piece (wear > 0)",
    );
}

#[test]
fn breaking_hit_surfaces_armor_broken_and_not_armor_damaged() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    let weapon = a_weapon(20, 12, 10, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut integrity = piece_integrity(1);
    let mut inflicted = InflictedWounds::default();
    assert!(
        *integrity > 0,
        "fixture: the near-broken piece must start protecting (1 > 0)",
    );

    let report = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     Some(struck_piece(0, 0, 0, ArmorType::Void, &mut integrity)),
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
    assert!(
        *integrity <= 0,
        "fixture: the breaking hit must leave the piece worn through (≤ 0)",
    );
    assert_eq!(
        applied.wear,
        ArmorWearOutcome::Broke(ArmorBroken::new(entity, part)),
        "a breaking hit must surface Broke for the struck ganger + part",
    );
}

#[test]
fn bare_flesh_hit_surfaces_neither_armor_signal() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    let weapon = a_weapon(14, 6, 4, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut integrity = piece_integrity(0);
    let mut inflicted = InflictedWounds::default();
    assert!(
        *integrity <= 0,
        "fixture: the struck piece must already be worn through (bare flesh)",
    );

    let report = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     Some(struck_piece(0, 0, 0, ArmorType::Void, &mut integrity)),
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
        applied.wear,
        ArmorWearOutcome::Unaffected,
        "a bare-flesh hit wears no piece — the wear outcome must be Unaffected \
         (neither the Broke nor the Damaged signal)",
    );
}
