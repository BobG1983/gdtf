use super::support::*;

#[test]
fn wounds_to_zero_is_dead() {
    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(3);
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
    let _broke = apply_hit(
        target,
        &hit(1, 0),
        Severity::Fatal,
        BodyPart::Torso,
        a_ganger(),
        &tuning,
    );

    assert_eq!(*wounds, 0, "Fatal must empty the Wounds pool");
    assert_eq!(
        life,
        LifeState::Dead,
        "Wounds depleted to 0 must set LifeState::Dead"
    );
}

#[test]
fn hp_to_zero_with_wounds_left_is_downed() {
    let mut hp = Hp::new(8);
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
    let _broke = apply_hit(
        target,
        &hit(20, 0),
        Severity::Minor,
        BodyPart::LeftLeg,
        a_ganger(),
        &tuning,
    );

    assert_eq!(
        *hp, 0,
        "an overshooting HP-loss must saturate Hp to 0 (no underflow)"
    );
    assert!(*wounds > 0, "a Minor wound must leave Wounds > 0");
    assert_eq!(
        life,
        LifeState::Downed,
        "Hp depleted to 0 with Wounds left must be Downed"
    );
}

#[test]
fn both_pools_depleted_is_dead_not_downed() {
    let mut hp = Hp::new(4);
    let mut wounds = Wounds::new(2);
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
    let _broke = apply_hit(
        target,
        &hit(99, 0),
        Severity::Fatal,
        BodyPart::Head,
        a_ganger(),
        &tuning,
    );

    assert_eq!(*hp, 0, "the lethal HP-loss must saturate Hp to 0");
    assert_eq!(*wounds, 0, "Fatal must empty the Wounds pool");
    assert_eq!(
        life,
        LifeState::Dead,
        "with BOTH pools depleted the ganger must be Dead (trumps Downed), not Downed",
    );
}

#[test]
fn corpse_skip_changes_nothing() {
    let mut hp = Hp::new(12);
    let mut wounds = Wounds::new(4);
    let mut life = LifeState::Dead;
    let mut integrity = worn_piece_integrity(1);
    let mut inflicted = InflictedWounds::default();
    let tuning = CombatTuning::default();

    let hp_before = hp;
    let wounds_before = wounds;
    let life_before = life;
    let integrity_before = integrity;

    let target = GangerHitTarget {
        hp:        &mut hp,
        wounds:    &mut wounds,
        life:      &mut life,
        integrity: Some(&mut integrity),
        inflicted: &mut inflicted,
    };
    let outcome = apply_hit(
        target,
        &hit(10, 50),
        Severity::Critical,
        BodyPart::Torso,
        a_ganger(),
        &tuning,
    );

    assert_eq!(
        outcome,
        ArmorWearOutcome::Unaffected,
        "a corpse-skip must emit no ArmorBroken / ArmorDamaged (Unaffected)"
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
        "a corpse-skip must record NO InflictedWound "
    );
}
