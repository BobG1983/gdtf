use super::support::*;

/// AC3 — bare-flesh path: wearing the struck piece to integrity ≤ 0 first
/// (`protects(part) == false`) makes the hit resolve with NO protection /
/// hardness. Asserts the report's matchup is `Neutral` (no armor type to match)
/// AND the HP-loss equals the FULL weapon damage (zeroed soak) — distinct from
/// the armored case on the same weapon, where protection soaks.
#[test]
fn bare_flesh_uses_no_protection_or_hardness() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::RightArm;
    // A weapon whose damage is small enough that real protection WOULD soak it
    // below itself — so the bare-flesh full-damage result is unmistakable.
    let weapon = a_weapon(10, 3, 0, DamageType::Kinetic);

    // --- Bare flesh: the struck piece is worn through (integrity 0). ---
    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    // integrity 0 ⇒ protects(part) == false ⇒ bare flesh, despite real
    // protection/hardness numbers on the (broken) piece.
    let mut worn = worn_suit(2, 20, 0, 5, ArmorType::Void);
    let mut inflicted = InflictedWounds::default();
    assert!(
        !worn.protects(part),
        "fixture: the struck piece must already be worn through (no protection)",
    );

    let report = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            worn:      &mut worn,
            inflicted: &mut inflicted,
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        },
        entity,
        &tuning,
        &mut rng(),
    );

    assert!(
        report.applied.is_some(),
        "a ganger hit must carry an applied-damage block",
    );
    let Some(applied) = report.applied else {
        return;
    };
    assert_eq!(
        applied.matchup,
        Matchup::Neutral,
        "bare flesh has no armor type to match → Neutral (no wheel advantage)",
    );
    // Zeroed soak ⇒ HP-loss == full weapon damage (no protection eats it).
    assert_eq!(
        *applied.hit.hp_damage, *weapon.damage,
        "bare flesh deals full weapon damage (no protection / hardness)",
    );
    assert_eq!(
        applied.broken, None,
        "bare flesh wears no piece — there is nothing to break",
    );

    // --- Armored counterpart: the same weapon vs a protecting piece soaks. ---
    let mut hp2 = Hp::new(50);
    let mut wounds2 = Wounds::new(9);
    let mut life2 = LifeState::Alive;
    // High protection, intact (protects == true) ⇒ the soak is in play.
    let mut worn2 = worn_suit(2, 20, 50, 5, ArmorType::Void);
    let mut inflicted2 = InflictedWounds::default();
    assert!(
        worn2.protects(part),
        "fixture: the armored piece must still protect",
    );
    let report2 = resolve_and_apply(
        &ganger_outcome(entity, part),
        weapon.stats(),
        Luck::new(0.0),
        TargetGanger {
            hp:        &mut hp2,
            wounds:    &mut wounds2,
            life:      &mut life2,
            worn:      &mut worn2,
            inflicted: &mut inflicted2,
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        },
        entity,
        &tuning,
        &mut rng(),
    );
    assert!(
        report2.applied.is_some(),
        "the armored hit must carry an applied-damage block",
    );
    let Some(applied2) = report2.applied else {
        return;
    };
    // The armored HP-loss is strictly below the bare-flesh full damage: real
    // protection soaked it. (Pins that bare flesh truly bypassed the soak.)
    assert!(
        *applied2.hit.hp_damage < *applied.hit.hp_damage,
        "the armored hit must take less HP-loss than bare flesh: {} >= {}",
        *applied2.hit.hp_damage,
        *applied.hit.hp_damage,
    );
}
