//! The terminal gates + corpse-skip (AC3–AC6): Wounds→0 is Dead, Hp→0 with Wounds
//! left is Downed, both pools depleted is Dead (trumps Downed), and a corpse-skip
//! mutates nothing.

use super::support::*;

/// AC3 — terminal gate Wounds depleted to 0 → Dead. Drains Wounds to 0 (here via
/// a Fatal hit, the cleanest pool-emptying path) at full HP and asserts the
/// ganger is `LifeState::Dead` — death even at full HP (the life pool is what
/// kills).
#[test]
fn wounds_to_zero_is_dead() {
    let mut hp = Hp::new(50); // full HP — death comes from the life pool, not HP
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

/// AC4 — terminal gate Hp depleted to 0 with Wounds remaining → Downed. Drives
/// HP to 0 with a non-Fatal tier (so Wounds stays > 0) and asserts the ganger is
/// `LifeState::Downed` (alive, incapacitated) — HP loss downs, never kills.
#[test]
fn hp_to_zero_with_wounds_left_is_downed() {
    let mut hp = Hp::new(8);
    let mut wounds = Wounds::new(5); // plenty left after a Minor spend
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
    // HP-loss (20) overshoots HP (8) — saturates to 0; Minor leaves Wounds > 0.
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

/// AC5 — Dead TRUMPS Downed: a single hit that depletes BOTH Hp → 0 AND
/// Wounds → 0 yields `Dead`, not `Downed`. A Fatal hit (empties Wounds) whose
/// HP-loss also overshoots HP — both gates trip, the Wounds gate must win.
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
    // HP-loss (99) overshoots HP (4) → 0, AND Fatal empties Wounds → 0.
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

/// AC6 — corpse-skip: applying a hit to an already-`Dead` ganger is a NO-OP on
/// every pool and on the worn armor (and emits no [`ArmorBroken`]). Snapshots all
/// four surfaces before, applies a hit that WOULD wear/deplete on a live ganger,
/// and asserts each is byte-for-byte unchanged.
#[test]
fn corpse_skip_changes_nothing() {
    let mut hp = Hp::new(12);
    let mut wounds = Wounds::new(4);
    let mut life = LifeState::Dead; // already a corpse
    let mut integrity = worn_piece_integrity(1); // near-broken: a live hit here WOULD break it
    let mut inflicted = InflictedWounds::default();
    let tuning = CombatTuning::default();

    // Snapshot every surface by value before applying.
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
    // A hit that, on a live ganger, would subtract HP, spend Wounds, and break
    // the near-broken piece — proving the skip, not a harmless input.
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
        "a corpse-skip must record NO InflictedWound (GTW-279)"
    );
}
