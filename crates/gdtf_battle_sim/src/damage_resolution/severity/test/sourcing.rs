//! Stats sourced off the entity/ledger into the roll (the E3.0 contract +
//! GTW-436 `effective_toughness`).

use bevy::ecs::world::World;

use super::{
    super::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    support::*,
};
use crate::{
    armor::BodyPart,
    ganger::{Luck, Toughness, effective_toughness},
    injuries::{
        GainedInjury, InflictedInjuries, InjuryEffect, InjuryName, InspectText, StatDelta,
        StatTarget,
    },
    resolve_hit::PenetratingDamage,
    tuning::SeverityScaling,
    weapon::FatalBias,
};

/// Contract — the defender's Toughness and both gangers' Luck are **sourced off
/// the entity** (the E3.0 components), then fed to `roll_severity`. A bare
/// `World` spawns a shooter and a defender each carrying the attribute
/// components, a `Query`/`World` access reads them back, and they drive the
/// roll — proving the stats come off the entity, not bare literals.
#[test]
fn stats_are_sourced_off_the_entity() {
    let scaling = SeverityScaling::default();

    let mut world = World::new();
    let shooter = world.spawn(Luck::new(2.0)).id();
    let defender = world.spawn((Toughness::new(3.0), Luck::new(4.0))).id();

    // Read the shooter's Luck off its entity.
    let mut shooter_q = world.query::<&Luck>();
    let shooter_read = shooter_q.get(&world, shooter);
    assert!(
        shooter_read.is_ok(),
        "shooter Luck must be queryable off the entity",
    );
    let Ok(&luck_shooter) = shooter_read else {
        return;
    };

    // Read the defender's Toughness + Luck off its entity.
    let mut defender_q = world.query::<(&Toughness, &Luck)>();
    let defender_read = defender_q.get(&world, defender);
    assert!(
        defender_read.is_ok(),
        "defender Toughness + Luck must be queryable off the entity",
    );
    let Ok((&toughness, &luck_defender)) = defender_read else {
        return;
    };

    // Feed the entity-sourced stats into the roll — the real read shape.
    let bundle = SeverityInputs::new(
        PenetratingDamage::new(10),
        toughness,
        part_severity_mod(BodyPart::Torso),
        FatalBias::new(0.0),
        luck_shooter,
        luck_defender,
    );
    let mut r = rng();
    let got = roll_severity(&bundle, &scaling, &mut r);
    // Value-agnostic: a real Severity came back from entity-sourced stats.
    assert!(
        Severity::ALL.contains(&got),
        "roll_severity must return a Severity from entity-sourced stats: {got:?}",
    );
}

/// GTW-436 C3 test (5) — a `Modify(Toughness)` injury, read through the gate-enforced
/// [`effective_toughness`] accessor, SHIFTS the NEXT severity roll: a big Toughness
/// debuff lowers the defender's mitigation, so the §6 score rises and the bucket rank
/// is non-decreasing across a fixed-seed pen sweep AND strictly higher at ≥ 1 pen.
/// Drives the REAL `roll_severity` over the REAL `effective_toughness` (not a
/// reimplementation). Pin-discriminating: reading the RAW Toughness (the divergence the
/// accessor closes) would feed the SAME mitigation in both arms, so the injured rank
/// could never exceed the baseline anywhere.
#[test]
fn toughness_injury_shifts_the_next_severity_roll() {
    let scaling = SeverityScaling::default();
    let base_toughness = Toughness::new(40.0);

    // The injured ledger: a heavy Toughness debuff (the effective accessor folds it in).
    let mut ledger = InflictedInjuries::default();
    ledger.gain(GainedInjury::new(
        InjuryName::new("battered".to_owned()),
        BodyPart::Torso,
        Severity::Major,
        vec![InjuryEffect::Modify {
            stat:   StatTarget::Toughness,
            amount: StatDelta::new(i8::MIN), // -128: a large mitigation loss
        }],
        InspectText::new("battered".to_owned()),
    ));

    // The effective value the §6 roll must see for the injured defender — through the
    // SINGLE gate-enforced read path (an empty ledger yields the base unchanged).
    let injured_toughness = effective_toughness(base_toughness, &ledger);
    let baseline_toughness = effective_toughness(base_toughness, &InflictedInjuries::default());
    assert_eq!(
        baseline_toughness, base_toughness,
        "an empty ledger leaves Toughness unchanged (the zero-delta identity)",
    );
    assert!(
        *injured_toughness < *base_toughness,
        "the Toughness debuff must lower the effective Toughness fed to the roll",
    );

    // Sweep pen at a FIXED seed (the roll term is held identical), comparing the baseline
    // (no injury) roll against the injured roll at each pen.
    let mut any_strictly_worse = false;
    for pen in [0, 4, 8, 12, 16, 24, 32, 48, 64, 96] {
        let part = part_severity_mod(BodyPart::Torso);
        let baseline = {
            let mut r = rng();
            roll_severity(
                &SeverityInputs::new(
                    PenetratingDamage::new(pen),
                    baseline_toughness,
                    part,
                    FatalBias::new(0.0),
                    Luck::new(0.0),
                    Luck::new(0.0),
                ),
                &scaling,
                &mut r,
            )
        };
        let injured = {
            let mut r = rng();
            roll_severity(
                &SeverityInputs::new(
                    PenetratingDamage::new(pen),
                    injured_toughness,
                    part,
                    FatalBias::new(0.0),
                    Luck::new(0.0),
                    Luck::new(0.0),
                ),
                &scaling,
                &mut r,
            )
        };
        assert!(
            injured.rank() >= baseline.rank(),
            "a lower effective Toughness must never lower the severity rank \
             (pen {pen}: baseline {baseline:?} -> injured {injured:?})",
        );
        if injured.rank() > baseline.rank() {
            any_strictly_worse = true;
        }
    }
    assert!(
        any_strictly_worse,
        "a Toughness injury must shift the severity roll outcome (a strictly worse bucket \
         at ≥ 1 pen) — proving effective_toughness routes the delta into the roll",
    );
}
