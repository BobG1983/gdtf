use super::support::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the reference side mirrors `resolve_and_apply`'s own inputs verbatim so the \
              equivalence is exact; bundling them would diverge the two sides' shapes"
)]
fn compose_by_hand(
    weapon: &WeaponBundle,
    part: BodyPart,
    piece: ArmorPiece,
    shooter_luck: Luck,
    toughness: Toughness,
    defender_luck: Luck,
    tuning: &CombatTuning,
    entity: Entity,
) -> (
    AppliedDamage,
    (Hp, Wounds, LifeState, ArmorIntegrity, InflictedWounds),
) {
    let mut hp = Hp::new(40);
    let mut wounds = Wounds::new(6);
    let mut life = LifeState::Alive;
    let mut integrity = piece.integrity;
    let mut inflicted = InflictedWounds::default();
    let mut rng_b = rng();

    let m = matchup(weapon.damage_type, piece.armor_type);
    let hit = resolve_hit(weapon.damage, weapon.punch, weapon.shred, &piece, m, tuning);
    let inputs = SeverityInputs::new(
        hit.penetrating,
        toughness,
        part_severity_mod(part),
        weapon.fatal_bias,
        shooter_luck,
        defender_luck,
    );
    let severity = roll_severity(&inputs, &tuning.severity_scaling, &mut rng_b);
    let wear = apply_hit(
        GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        },
        &hit,
        severity,
        part,
        entity,
        tuning,
    );
    let applied = AppliedDamage {
        matchup: m,
        hit,
        severity,
        life_after: life,
        wear,
    };
    (applied, (hp, wounds, life, integrity, inflicted))
}

#[test]
fn fold_equals_the_composed_steps() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    let weapon = a_weapon(14, 10, 6, DamageType::Kinetic);
    let shooter_luck = Luck::new(2.0);
    let outcome = ganger_outcome(entity, part);

    let (floor, prot, integ, hard, at) = (1, 8, 30, 2, ArmorType::Void);
    let toughness = Toughness::new(3.0);
    let defender_luck = Luck::new(4.0);

    let mut hp_a = Hp::new(40);
    let mut wounds_a = Wounds::new(6);
    let mut life_a = LifeState::Alive;
    let mut integrity_a = piece_integrity(integ);
    let mut inflicted_a = InflictedWounds::default();
    let mut rng_a = rng();
    let piece_a = Some(struck_piece(floor, prot, hard, at, &mut integrity_a));
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        shooter_luck,
        Some(TargetGanger {
            hp: &mut hp_a,
            wounds: &mut wounds_a,
            life: &mut life_a,
            piece: piece_a,
            inflicted: &mut inflicted_a,
            toughness,
            luck: defender_luck,
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut rng_a,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    );

    let piece = ArmorPiece::new(
        ArmorFloor::new(floor),
        ArmorProtection::new(prot),
        ArmorIntegrity::new(integ),
        ArmorHardness::new(hard),
        at,
    );
    let (applied_b, (hp_b, wounds_b, life_b, integrity_b, inflicted_b)) = compose_by_hand(
        &weapon,
        part,
        piece,
        shooter_luck,
        toughness,
        defender_luck,
        &tuning,
        entity,
    );

    assert_eq!(
        applied_of(&report),
        Some(applied_b),
        "the folded report must equal the composed matchup/hit/severity/state",
    );
    assert_eq!(
        ganger_verdict(&report).map(|v| v.part),
        Some(part),
        "the verdict carries the struck part"
    );

    assert_eq!(
        (hp_a, wounds_a, life_a, integrity_a, &inflicted_a),
        (hp_b, wounds_b, life_b, integrity_b, &inflicted_b),
        "every mutated surface after the fold must equal the hand-composed steps \
         (Hp / Wounds / LifeState / struck-piece integrity / InflictedWounds)",
    );
}

#[test]
fn armored_report_carries_the_real_matchup() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;

    let resolve = |weapon: WeaponBundle, armor_type: ArmorType| {
        let mut hp = Hp::new(50);
        let mut wounds = Wounds::new(9);
        let mut life = LifeState::Alive;
        let mut integrity = piece_integrity(40);
        let mut inflicted = InflictedWounds::default();
        resolve_and_apply(
            &ganger_outcome(entity, part),
            weapon.stats(),
            Luck::new(0.0),
            Some(TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                piece:     Some(struck_piece(1, 10, 1, armor_type, &mut integrity)),
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
        )
    };

    let weapon = a_weapon(12, 10, 6, DamageType::Kinetic);
    let fav = resolve(weapon.clone(), ArmorType::Refractive);
    let res = resolve(weapon, ArmorType::Plated);

    assert!(
        applied_of(&fav).is_some() && applied_of(&res).is_some(),
        "both armored hits must carry an applied block",
    );
    let (Some(fav_a), Some(res_a)) = (applied_of(&fav), applied_of(&res)) else {
        return;
    };
    assert_eq!(
        fav_a.matchup,
        Matchup::Favorable,
        "Kinetic vs Refractive must resolve Favorable in the report",
    );
    assert!(
        *fav_a.hit.penetrating >= *res_a.hit.penetrating,
        "a Favorable matchup must yield >= penetrating damage than Resisted",
    );
}
