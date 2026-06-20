//! HP-loss + Wounds-by-tier folding (AC1 / AC2): a graze bruises HP and spends no
//! Wound, Fatal empties the pool, and the tier costs ascend Minor < Major < Critical.

use super::support::*;

/// AC1 — HP loss applies ALWAYS, even on a `Severity::None` graze, and no Wound
/// is spent. Drives a graze with `hp_damage > 0` onto a healthy ganger and
/// asserts HP fell by exactly the HP-loss while Wounds is unchanged.
#[test]
fn graze_subtracts_hp_and_spends_no_wound() {
    let mut hp = Hp::new(20);
    let mut wounds = Wounds::new(5);
    let mut life = LifeState::Alive;
    let mut worn = worn_suit(100);
    let mut inflicted = InflictedWounds::default();
    let tuning = CombatTuning::default();

    let target = GangerHitTarget {
        hp:        &mut hp,
        wounds:    &mut wounds,
        life:      &mut life,
        worn:      &mut worn,
        inflicted: &mut inflicted,
    };
    // A graze: Severity::None, HP-loss 7, low wear (wears but does not break).
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
        "a graze (Severity::None) registers no Wound, so it must record NO InflictedWound (GTW-279 AC3)"
    );
    assert_eq!(
        life,
        LifeState::Alive,
        "a non-lethal graze must leave the ganger Alive"
    );
    // A low-wear hit on a fresh suit wears the piece (GTW-313 Worn(delta=1)) but must
    // NOT break it — it surfaces Worn, never Broke.
    assert_eq!(
        outcome,
        ArmorWearOutcome::Worn(ArmorWorn::new(
            ganger,
            BodyPart::Torso,
            IntegrityWear::new(1)
        )),
        "a low-wear hit on a fresh suit must wear (Worn), never break (Broke)"
    );
}

/// AC2 (Fatal empties) — a `Severity::Fatal` hit drives `Wounds == 0` regardless
/// of the prior pool (the mechanism), independent of any tuned cost. Uses a large
/// starting pool so a per-tier subtraction could never reach 0 — only the
/// pool-emptying branch can.
#[test]
fn fatal_empties_the_wounds_pool_regardless_of_prior() {
    let tuning = CombatTuning::default();
    for prior in [1u8, 3, 7, 200, u8::MAX] {
        let mut hp = Hp::new(50);
        let mut wounds = Wounds::new(prior);
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(100);
        let mut inflicted = InflictedWounds::default();

        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            worn:      &mut worn,
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

/// AC2 (tier ordering) — Minor < Major < Critical in Wounds cost, as a RELATION
/// (never a pinned split). Constructs three otherwise-identical hits differing
/// only in their severity tier, applies each to an identical fresh ganger, and
/// asserts the post-application Wounds order: more severe ⇒ fewer Wounds left.
#[test]
fn wound_cost_orders_minor_lt_major_lt_critical() {
    let tuning = CombatTuning::default();
    // A pool large enough that even Critical's cost can't reach 0 (so the gate
    // never collapses the ordering to a shared floor) — an arbitrary fixture.
    let start = Wounds::new(200);

    let post_wounds = |severity: Severity| -> u8 {
        let mut hp = Hp::new(50);
        let mut wounds = start;
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(100);
        let mut inflicted = InflictedWounds::default();
        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            worn:      &mut worn,
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

    // More severe ⇒ spends MORE Wounds ⇒ leaves FEWER — Minor < Major < Critical
    // cost, asserted as the relation, never the exact split.
    assert!(
        after_minor > after_major,
        "Major must spend more Wounds than Minor: {after_minor} (minor) <= {after_major} (major)",
    );
    assert!(
        after_major > after_critical,
        "Critical must spend more Wounds than Major: {after_major} (major) <= {after_critical} (critical)",
    );
}
