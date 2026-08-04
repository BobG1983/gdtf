use super::support::*;

#[test]
fn graze_subtracts_hp_and_spends_no_wound() {
    let mut hp = Hp::new(20);
    let mut wounds = Wounds::new(5);
    let mut life = LifeState::Alive;
    let mut integrity = worn_piece_integrity(100);
    let mut inflicted = InflictedWounds::default();
    let tuning = CombatTuning::default();

    let target = GangerHitTarget {
        hp:        &mut hp,
        wounds:    &mut wounds,
        life:      &mut life,
        integrity: Some(&mut integrity),
        inflicted: &mut inflicted,
    };
    let ganger = a_ganger();
    let outcome = apply_hit(
        target,
        &hit(7, 1),
        Severity::None,
        BodyPart::Torso,
        ganger,
        &tuning,
    );

    assert_eq!(*hp, 20 - 7, "a graze must subtract its HP-loss from Hp");
    assert_eq!(*wounds, 5, "a graze (Severity::None) must spend NO Wound");
    assert!(
        inflicted.is_empty(),
        "a graze (Severity::None) registers no Wound, so it must record NO InflictedWound "
    );
    assert_eq!(
        life,
        LifeState::Alive,
        "a non-lethal graze must leave the ganger Alive"
    );
    assert_eq!(
        outcome,
        ArmorWearOutcome::Damaged(ArmorDamaged::new(
            ganger,
            BodyPart::Torso,
            IntegrityWear::new(1)
        )),
        "a low-wear hit on a fresh suit must wear (Damaged), never break (Broke)"
    );
}

#[test]
fn fatal_empties_the_wounds_pool_regardless_of_prior() {
    let tuning = CombatTuning::default();
    for prior in [1u8, 3, 7, 200, u8::MAX] {
        let mut hp = Hp::new(50);
        let mut wounds = Wounds::new(prior);
        let mut life = LifeState::Alive;
        let mut integrity = worn_piece_integrity(100);
        let mut inflicted = InflictedWounds::default();

        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        };
        let _broke = apply_hit(
            target,
            &hit(1, 0),
            Severity::Fatal,
            BodyPart::Head,
            a_ganger(),
            &tuning,
        );

        assert_eq!(
            *wounds, 0,
            "Fatal must empty the Wounds pool from prior {prior}"
        );
        assert_eq!(
            inflicted.as_slice(),
            &[InflictedWound::new(Severity::Fatal, BodyPart::Head)],
            "a Fatal hit registers a wound, so it must record one InflictedWound (Fatal/Head)"
        );
    }
}

#[test]
fn wound_cost_orders_minor_lt_major_lt_critical() {
    let tuning = CombatTuning::default();
    let start = Wounds::new(200);

    let post_wounds = |severity: Severity| -> u8 {
        let mut hp = Hp::new(50);
        let mut wounds = start;
        let mut life = LifeState::Alive;
        let mut integrity = worn_piece_integrity(100);
        let mut inflicted = InflictedWounds::default();
        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        };
        let _broke = apply_hit(
            target,
            &hit(1, 0),
            severity,
            BodyPart::Torso,
            a_ganger(),
            &tuning,
        );
        *wounds
    };

    let after_minor = post_wounds(Severity::Minor);
    let after_major = post_wounds(Severity::Major);
    let after_critical = post_wounds(Severity::Critical);

    assert!(
        after_minor > after_major,
        "Major must spend more Wounds than Minor: {after_minor} (minor) <= {after_major} (major)",
    );
    assert!(
        after_major > after_critical,
        "Critical must spend more Wounds than Major: {after_major} (major) <= {after_critical} (critical)",
    );
}
