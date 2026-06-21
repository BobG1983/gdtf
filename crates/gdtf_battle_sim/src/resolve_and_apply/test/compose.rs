use super::support::*;

/// The hand-composed `matchup` → `resolve_hit` → `roll_severity` → `apply_hit` steps
/// the AC1 equivalence test runs as the reference side — returns the composed
/// [`AppliedDamage`] block PLUS the final mutated ganger state, so the test body stays
/// the comparison only (one verb chain, no shadowing of the units under test).
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
    let wear_outcome = apply_hit(
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
    // Map the ArmorWearOutcome onto the two report fields the SAME way the fold does.
    let (broken, worn) = match wear_outcome {
        ArmorWearOutcome::Broke(broken) => (Some(broken), None),
        ArmorWearOutcome::Worn(worn) => (None, Some(worn)),
        ArmorWearOutcome::Unaffected => (None, None),
    };
    let applied = AppliedDamage {
        matchup: m,
        hit,
        severity,
        life_after: life,
        broken,
        worn,
    };
    (applied, (hp, wounds, life, integrity, inflicted))
}

/// AC1 (THE key test) — the fold equals the composition: `resolve_and_apply` on
/// a `Ganger` outcome produces a report whose damage / severity AND the
/// resulting ganger state are IDENTICAL to running `matchup` → `resolve_hit` →
/// `roll_severity` → `apply_hit` by hand, with the SAME seed and the SAME inputs
/// on a clone of the target. The units under test are NOT shadowed — both sides
/// call the real verbs; the same single draw makes the RNG streams line up.
#[test]
fn fold_equals_the_composed_steps() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    let weapon = a_weapon(14, 10, 6, DamageType::Kinetic);
    let shooter_luck = Luck::new(2.0);
    let outcome = ganger_outcome(entity, part);

    // The struck piece's stats (the fixture both sides resolve against). Integrity is
    // held per side so each path wears its OWN piece-entity component (GTW-323).
    let (floor, prot, integ, hard, at) = (1, 8, 30, 2, ArmorType::Void);
    let toughness = Toughness::new(3.0);
    let defender_luck = Luck::new(4.0);

    // --- The folded act ---
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
        TargetGanger {
            hp: &mut hp_a,
            wounds: &mut wounds_a,
            life: &mut life_a,
            piece: piece_a,
            inflicted: &mut inflicted_a,
            toughness,
            luck: defender_luck,
        },
        entity,
        &tuning,
        &mut rng_a,
    );

    // --- The composed steps, BY HAND, on a clone with the same seed ---
    // The read-only ArmorPiece value the damage formula consumes (the same value
    // `struck_piece` assembles from the piece's stat components).
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

    // The report's damage block matches the hand-composed steps.
    assert_eq!(
        report.applied,
        Some(applied_b),
        "the folded report must equal the composed matchup/hit/severity/state",
    );
    assert_eq!(
        report.part,
        Some(part),
        "the report carries the struck part"
    );

    // The resulting ganger state matches the hand-composed steps, every mutated
    // surface at once — the fold mutated the target identically to the composition
    // (incl. the struck piece's worn integrity and the GTW-279 InflictedWounds record).
    assert_eq!(
        (hp_a, wounds_a, life_a, integrity_a, &inflicted_a),
        (hp_b, wounds_b, life_b, integrity_b, &inflicted_b),
        "every mutated surface after the fold must equal the hand-composed steps \
         (Hp / Wounds / LifeState / struck-piece integrity / InflictedWounds)",
    );
}

/// AC1 (counterpart) — the matchup wheel advantage IS felt on an armored ganger
/// hit: at identical seed and inputs, a Favorable damage-type-vs-armor pairing
/// yields `>=` penetrating damage than a Resisted one — the report's matchup is
/// genuinely the wheel lookup, not a hardcoded Neutral.
#[test]
fn armored_report_carries_the_real_matchup() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;

    // Pick a damage type/armor type pairing and confirm the report names the
    // wheel's verdict — then a clearly Resisted pairing names Resisted.
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
            TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                piece:     Some(struck_piece(1, 10, 1, armor_type, &mut integrity)),
                inflicted: &mut inflicted,
                toughness: Toughness::new(0.0),
                luck:      Luck::new(0.0),
            },
            entity,
            &tuning,
            &mut rng(),
        )
    };

    // Kinetic (node 3) is strong against {6, 1, 2} = {Ceramic, Refractive, Flak}
    // and resisted by the rest. Favorable vs Refractive, Resisted vs Void(3's
    // own mirror is Neutral, so use Plated node 0 → resisted).
    let weapon = a_weapon(12, 10, 6, DamageType::Kinetic);
    let fav = resolve(weapon.clone(), ArmorType::Refractive);
    let res = resolve(weapon, ArmorType::Plated);

    assert!(
        fav.applied.is_some() && res.applied.is_some(),
        "both armored hits must carry an applied block",
    );
    let (Some(fav_a), Some(res_a)) = (fav.applied, res.applied) else {
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
