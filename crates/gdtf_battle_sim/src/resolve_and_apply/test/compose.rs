use super::support::*;

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

    // --- The folded act ---
    let mut hp_a = Hp::new(40);
    let mut wounds_a = Wounds::new(6);
    let mut life_a = LifeState::Alive;
    let mut worn_a = worn_suit(1, 8, 30, 2, ArmorType::Void);
    let toughness = Toughness::new(3.0);
    let defender_luck = Luck::new(4.0);
    let mut rng_a = rng();
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        shooter_luck,
        TargetGanger {
            hp: &mut hp_a,
            wounds: &mut wounds_a,
            life: &mut life_a,
            worn: &mut worn_a,
            toughness,
            luck: defender_luck,
        },
        entity,
        &tuning,
        &mut rng_a,
    );

    // --- The composed steps, BY HAND, on a clone with the same seed ---
    let mut hp_b = Hp::new(40);
    let mut wounds_b = Wounds::new(6);
    let mut life_b = LifeState::Alive;
    let mut worn_b = worn_suit(1, 8, 30, 2, ArmorType::Void);
    let mut rng_b = rng();

    let piece = worn_b.at(part);
    let m = matchup(weapon.damage_type, piece.armor_type);
    let hit = resolve_hit(
        weapon.damage,
        weapon.punch,
        weapon.shred,
        &piece,
        m,
        &tuning,
    );
    let inputs = SeverityInputs::new(
        hit.penetrating,
        toughness,
        part_severity_mod(part),
        weapon.fatal_bias,
        shooter_luck,
        defender_luck,
    );
    let severity = roll_severity(&inputs, &tuning.severity_scaling, &mut rng_b);
    let broken = apply_hit(
        GangerHitTarget {
            hp:     &mut hp_b,
            wounds: &mut wounds_b,
            life:   &mut life_b,
            worn:   &mut worn_b,
        },
        &hit,
        severity,
        part,
        entity,
        &tuning,
    );

    // The report's damage block matches the hand-composed steps.
    assert_eq!(
        report.applied,
        Some(AppliedDamage {
            matchup: m,
            hit,
            severity,
            life_after: life_b,
            broken,
        }),
        "the folded report must equal the composed matchup/hit/severity/state",
    );
    assert_eq!(
        report.part,
        Some(part),
        "the report carries the struck part"
    );

    // The resulting ganger state matches the hand-composed steps, surface by
    // surface — the fold mutated the target identically to the composition.
    assert_eq!(hp_a, hp_b, "HP after the fold must equal the composed HP");
    assert_eq!(
        wounds_a, wounds_b,
        "Wounds after the fold must equal the composed Wounds",
    );
    assert_eq!(
        life_a, life_b,
        "LifeState after the fold must equal the composed LifeState",
    );
    assert_eq!(
        worn_a, worn_b,
        "WornArmor after the fold must equal the composed WornArmor",
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
        let mut worn = worn_suit(1, 10, 40, 1, armor_type);
        resolve_and_apply(
            &ganger_outcome(entity, part),
            weapon.stats(),
            Luck::new(0.0),
            TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                worn:      &mut worn,
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
