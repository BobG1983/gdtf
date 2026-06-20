//! GTW-313 — the report's mutually-exclusive armor signals through the REAL fold
//! ([`resolve_and_apply`](super::super::resolve_and_apply)): a wearing-not-breaking
//! hit surfaces `worn` (carrying the integrity delta) and NOT `broken`; a breaking
//! hit surfaces `broken` and NOT `worn`; and a hit on a bare / worn-through piece
//! surfaces NEITHER. Asserts on the frozen report, never a copy of the unit.

use super::support::*;

/// A wearing-but-not-breaking hit on an intact piece surfaces `ArmorWorn` carrying
/// the per-hit integrity delta and leaves `broken` `None` — the GTW-313 reduction
/// signal, mutually exclusive with the break signal.
#[test]
fn wearing_hit_surfaces_armor_worn_with_the_delta_and_no_break() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    // A weapon that wears the piece but cannot break a high-integrity suit in one hit.
    let weapon = a_weapon(10, 4, 0, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    // High integrity (100) so the single hit's wear leaves it well above zero.
    let mut worn = worn_suit(2, 8, 100, 1, ArmorType::Void);
    let mut inflicted = InflictedWounds::default();
    assert!(
        worn.protects(part),
        "fixture: the struck piece must start protecting",
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
    assert!(
        worn.protects(part),
        "fixture: a non-breaking hit must leave the piece still protecting",
    );
    // The Worn signal carries the EXACT delta the hit's wear computed (the report's
    // own HitResult::wear), proving the surfaced delta == the removed integrity.
    assert_eq!(
        applied.worn,
        Some(ArmorWorn::new(entity, part, applied.hit.wear)),
        "a wearing-not-breaking hit must surface ArmorWorn carrying the hit's wear delta",
    );
    assert!(
        *applied.hit.wear > 0,
        "fixture: the hit must actually have worn the piece (wear > 0)",
    );
    assert_eq!(
        applied.broken, None,
        "a wearing-not-breaking hit must NOT surface ArmorBroken (mutually exclusive)",
    );
}

/// A breaking hit (wears a near-broken piece past zero) surfaces `ArmorBroken` and
/// leaves `worn` `None` — the break signal, mutually exclusive with the wear signal.
#[test]
fn breaking_hit_surfaces_armor_broken_and_not_armor_worn() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    // A heavy-shred weapon to drive the wear past a near-broken piece.
    let weapon = a_weapon(20, 12, 10, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    // Integrity 1: protecting, but one wearing hit crosses it to ≤ 0.
    let mut worn = worn_suit(0, 0, 1, 0, ArmorType::Void);
    let mut inflicted = InflictedWounds::default();
    assert!(
        worn.protects(part),
        "fixture: the near-broken piece must start protecting (1 > 0)",
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
    assert!(
        !worn.protects(part),
        "fixture: the breaking hit must leave the piece worn through (≤ 0)",
    );
    assert_eq!(
        applied.broken,
        Some(ArmorBroken::new(entity, part)),
        "a breaking hit must surface ArmorBroken for the struck ganger + part",
    );
    assert_eq!(
        applied.worn, None,
        "a breaking hit must NOT surface ArmorWorn (mutually exclusive with broken)",
    );
}

/// A hit on a worn-through (bare-flesh) piece surfaces NEITHER armor signal — there
/// is nothing left to wear or break (GTW-313 Unaffected through the fold).
#[test]
fn bare_flesh_hit_surfaces_neither_armor_signal() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let part = BodyPart::Torso;
    let weapon = a_weapon(14, 6, 4, DamageType::Kinetic);

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    // Integrity 0 ⇒ already worn through ⇒ bare flesh (protects == false).
    let mut worn = worn_suit(0, 0, 0, 0, ArmorType::Void);
    let mut inflicted = InflictedWounds::default();
    assert!(
        !worn.protects(part),
        "fixture: the struck piece must already be worn through (bare flesh)",
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
        applied.broken, None,
        "a bare-flesh hit breaks no piece — broken must be None",
    );
    assert_eq!(
        applied.worn, None,
        "a bare-flesh hit wears no piece — worn must be None (surfaces NEITHER signal)",
    );
}
