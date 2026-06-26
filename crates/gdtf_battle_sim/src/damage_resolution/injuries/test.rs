//! Vocabulary tests for the GTW-435 injury types — the [`InflictedInjuries::gain`]
//! ledger folding (C3) and the `.injury.ron` deserialize (C4).
//!
//! These exercise the REAL public API on the real types (no stubs): `gain` folds
//! through the actual [`StatDeltaLedger`] / [`BleedAfflicted`] accumulators, and the
//! RON test parses through the actual `serde` derive. Per the loader-tests rule, the
//! RON test asserts STRUCTURE (kinds resolve, the shape round-trips), never specific
//! tunable magnitudes.

use super::{
    BleedAfflicted, BleedAmount, GainedInjury, HandsAvailable, InflictedInjuries, InjuryDef,
    InjuryEffect, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight, InspectText, LogText,
    PopupText, PostHeal, StatDelta, StatDeltaSum, StatKind, StatTarget, WeightedInjuryEntry,
    WeightedInjuryTable, roll_injury,
};
use crate::{
    armor::BodyPart,
    rng::{BattleSeed, InjuryRng},
    severity::Severity,
};

/// Build a minimal [`GainedInjury`] carrying the given effects (the texts and the
/// part/severity are irrelevant to the folding under test).
fn gained_with(effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        BodyPart::Head,
        Severity::Minor,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

/// Build a [`GainedInjury`] on a chosen `part` carrying the given effects — the
/// part-parameterized fixture the GTW-443 hand-count fold tests need (the disabled side
/// is derived from `part`).
fn gained_on(part: BodyPart, effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        part,
        Severity::Major,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

// ── GTW-443: DisableHand inert gain (C2) + per-side hands_available fold (C3) ──

#[test]
fn gain_of_disable_hand_is_inert_no_delta_no_bleed() {
    // C2: a DisableHand-bearing injury folds INERTLY — it accumulates NO stat delta and
    // NO bleed by that effect alone. (The hand count is surfaced by the read-fold, not a
    // stored accumulator.) Pin-discriminating: a stored-counter implementation would
    // dirty some StatDeltaSum or the bleed here.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));

    // Every stat delta stays at its zero default, and the bleed accrual is untouched.
    for stat in StatTarget::ALL {
        assert_eq!(
            ledger.delta_for(stat),
            StatDeltaSum::default(),
            "DisableHand must not dock {stat:?}"
        );
    }
    assert_eq!(
        ledger.bleed(),
        BleedAfflicted::default(),
        "DisableHand must accrue no bleed"
    );
    // The ledger still RECORDS the injury (the fold reads it).
    assert_eq!(ledger.gained().len(), 1);
}

#[test]
fn hands_available_none_is_two() {
    // C3(a): no arm injury → both hands available.
    let ledger = InflictedInjuries::default();
    assert_eq!(ledger.hands_available(), HandsAvailable::new(2));
}

#[test]
fn hands_available_one_left_arm_disable_is_one() {
    // C3(b): one LeftArm DisableHand → exactly one hand left.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    assert_eq!(ledger.hands_available(), HandsAvailable::new(1));
}

#[test]
fn two_same_side_disable_hands_still_leave_one_hand() {
    // C3(c) — THE SHARPEST PIN: two LeftArm DisableHand injuries disable exactly ONE
    // hand, leaving 1 (NOT 0). A naive injury-count subtraction (2 - 2 = 0) would FAIL
    // this; the set-over-distinct-sides fold gives 1. Order-independent.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    assert_eq!(
        ledger.hands_available(),
        HandsAvailable::new(1),
        "two SAME-side DisableHand injuries disable exactly one hand (set over sides, \
         not a count over entries)"
    );
}

#[test]
fn both_arms_disabled_is_zero_hands() {
    // C3(d): LeftArm + RightArm DisableHand → zero hands.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    ledger.gain(gained_on(
        BodyPart::RightArm,
        vec![InjuryEffect::DisableHand],
    ));
    assert_eq!(ledger.hands_available(), HandsAvailable::new(0));
}

#[test]
fn disable_hand_on_non_arm_part_is_inert_for_hand_count() {
    // C3(e): a DisableHand carried by a non-arm injury (Torso here) is INERT for the hand
    // count — there is no hand to disable. Both hands remain.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(BodyPart::Torso, vec![InjuryEffect::DisableHand]));
    assert_eq!(
        ledger.hands_available(),
        HandsAvailable::new(2),
        "a DisableHand on a non-arm part disables no hand"
    );
}

#[test]
fn disable_hand_effect_deserializes_from_ron() {
    // C1-adjacent: the fieldless DisableHand variant + a sibling Modify(Shooting) parse
    // from the same effects Vec — the .injury.ron shape the hand-disabling injuries use.
    let ron = r#"(
        name:         "Shattered Left Hand",
        body_part:    LeftArm,
        severity:     Major,
        popup_text:   "LEFT HAND SHATTERED",
        log_text:     "shatters a hand",
        inspect_text: "Shattered Left Hand",
        effects: [
            DisableHand,
            Modify(stat: Shooting, amount: -2),
        ],
    )"#;
    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "the DisableHand + Modify effects Vec must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.body_part, BodyPart::LeftArm);
    assert!(matches!(def.effects[0], InjuryEffect::DisableHand));
    assert!(matches!(
        def.effects[1],
        InjuryEffect::Modify {
            stat: StatTarget::Shooting,
            ..
        }
    ));
}

#[test]
fn stat_target_kind_splits_first_eight_attribute_rest_derived() {
    // The first eight ALL entries are attributes, the last eight are derived.
    for (i, stat) in StatTarget::ALL.into_iter().enumerate() {
        let expected = if i < 8 {
            StatKind::Attribute
        } else {
            StatKind::Derived
        };
        assert_eq!(
            stat.kind(),
            expected,
            "stat at index {i} has the wrong kind"
        );
        assert_eq!(stat.index(), i, "stat index must match its ALL position");
    }
    assert_eq!(StatTarget::COUNT, 16);
}

#[test]
fn gain_folds_a_modify_on_a_direct_attribute() {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Aim,
        amount: StatDelta::new(-2),
    }]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-2));
    // Unmentioned stats stay zero.
    assert_eq!(ledger.delta_for(StatTarget::Cool), StatDeltaSum::default());
    assert_eq!(ledger.gained().len(), 1);
}

#[test]
fn gain_folds_a_modify_on_a_derived_stat() {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Shooting,
        amount: StatDelta::new(-1),
    }]));

    assert_eq!(StatTarget::Shooting.kind(), StatKind::Derived);
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    );
}

#[test]
fn gain_folds_a_bleeding_effect_into_the_bleed_accrual() {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_with(vec![InjuryEffect::Bleeding {
        amount: BleedAmount::new(3),
    }]));

    assert_eq!(ledger.bleed(), BleedAfflicted::new(3));
    // A Bleeding effect contributes no stat delta.
    assert_eq!(ledger.delta_for(StatTarget::Hp), StatDeltaSum::default());
}

#[test]
fn gain_folds_a_multi_effect_injury_and_stacks_across_injuries() {
    let mut ledger = InflictedInjuries::default();

    // A single multi-effect injury: two Modifies (one attribute, one derived) plus a
    // Bleeding — all fold from ONE gain().
    ledger.gain(gained_with(vec![
        InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-2),
        },
        InjuryEffect::Modify {
            stat:   StatTarget::Shooting,
            amount: StatDelta::new(-1),
        },
        InjuryEffect::Bleeding {
            amount: BleedAmount::new(2),
        },
    ]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-2));
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    );
    assert_eq!(ledger.bleed(), BleedAfflicted::new(2));
    assert_eq!(ledger.gained().len(), 1);

    // A second injury stacking on the SAME stat ADDS, and a second bleed ACCRUES.
    ledger.gain(gained_with(vec![
        InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-3),
        },
        InjuryEffect::Bleeding {
            amount: BleedAmount::new(1),
        },
    ]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-5)); // -2 + -3
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    ); // untouched
    assert_eq!(ledger.bleed(), BleedAfflicted::new(3)); // 2 + 1
    assert_eq!(ledger.gained().len(), 2); // ordered ledger grows
}

#[test]
fn gain_appends_to_the_ledger_in_infliction_order() {
    let mut ledger = InflictedInjuries::default();
    let first = gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Grit,
        amount: StatDelta::new(-1),
    }]);
    let second = gained_with(vec![InjuryEffect::Bleeding {
        amount: BleedAmount::new(1),
    }]);

    ledger.gain(first.clone());
    ledger.gain(second.clone());

    assert_eq!(ledger.gained(), &[first, second]);
}

#[test]
fn injury_def_deserializes_from_the_schema_ron() {
    // The design-doc schema sample. Structure-only assertions (loader-tests rule:
    // no pinned tunable magnitudes — assert the SHAPE, not specific weights/amounts).
    let ron = r#"(
        name:         "Lost Eye",
        body_part:    Head,
        severity:     Critical,
        popup_text:   "LOST EYE",
        log_text:     "loses an eye",
        inspect_text: "Lost Eye -- -2 Aim, -1 Cool",
        effects: [
            Modify(stat: Aim,  amount: -2),
            Modify(stat: Cool, amount: -1),
        ],
        post_heal:    Deferred,
    )"#;

    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "schema RON must deserialize into InjuryDef: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };

    assert_eq!(def.body_part, BodyPart::Head);
    assert_eq!(def.severity, Severity::Critical);
    assert_eq!(def.post_heal, PostHeal::Deferred);
    assert_eq!(def.effects.len(), 2);
    // Both effects parsed as Modify on the named stats (structure, not magnitude).
    assert!(matches!(
        def.effects[0],
        InjuryEffect::Modify {
            stat: StatTarget::Aim,
            ..
        }
    ));
    assert!(matches!(
        def.effects[1],
        InjuryEffect::Modify {
            stat: StatTarget::Cool,
            ..
        }
    ));
}

#[test]
fn injury_def_post_heal_defaults_to_deferred_when_omitted() {
    // post_heal is parsed-but-unread (C4): a floor file may omit it entirely.
    let ron = r#"(
        name:         "Scalp Graze",
        body_part:    Head,
        severity:     Minor,
        popup_text:   "SCALP GRAZE",
        log_text:     "is grazed across the scalp",
        inspect_text: "Scalp Graze",
        effects: [ Bleeding(amount: 1) ],
    )"#;

    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "RON without post_heal must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.post_heal, PostHeal::Deferred);
    assert!(matches!(def.effects[0], InjuryEffect::Bleeding { .. }));
}

// ── GTW-438: roll_injury draw-discipline + determinism ───────────────────────

/// A fixed seed for the roll tests (arbitrary, not tuned).
const ROLL_SEED: u64 = 0x90A1_C401;

/// A fresh [`InjuryRng`] from the shared roll seed.
fn injury_rng() -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(ROLL_SEED))
}

/// Build a single-entry registry + table for `(part, severity)` keyed by `key`, so a
/// roll for that bucket resolves a known [`InjuryDef`]. The def carries one `Modify`
/// effect (the magnitude is irrelevant — the tests assert STRUCTURE, not tuning).
fn one_injury_table(
    key: &str,
    part: BodyPart,
    severity: Severity,
) -> (InjuryRegistry, InjuryTables) {
    let name = InjuryName::new(key.to_owned());
    let def = InjuryDef {
        name: name.clone(),
        body_part: part,
        severity,
        popup_text: PopupText::new("HURT".to_owned()),
        log_text: LogText::new("is hurt".to_owned()),
        inspect_text: InspectText::new("Hurt".to_owned()),
        effects: vec![InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-2),
        }],
        post_heal: PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(name.clone(), def)]);
    let mut tables = InjuryTables::default();
    tables.insert(
        part,
        severity,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );
    (registry, tables)
}

#[test]
fn roll_injury_takes_no_draw_for_none_or_fatal() {
    // C1/C2 + stream-alignment (#2): a graze (None) and a Fatal take NO InjuryRng draw and
    // return None. Proof: the rng cursor is UNMOVED — its next raw u64 equals a fresh
    // stream's next raw u64.
    let (registry, tables) = one_injury_table("hurt", BodyPart::Head, Severity::Minor);
    for severity in [Severity::None, Severity::Fatal] {
        let mut rng = injury_rng();
        let rolled = roll_injury(BodyPart::Head, severity, &tables, &registry, &mut rng);
        assert!(
            rolled.is_none(),
            "{severity:?} is not tabled — no injury rolled"
        );
        // The cursor did NOT advance: next draw equals a fresh stream's first draw.
        assert_eq!(
            rng.next_u64(),
            injury_rng().next_u64(),
            "{severity:?} must take NO InjuryRng draw (the cursor stays put)"
        );
    }
}

#[test]
fn roll_injury_draws_exactly_one_and_resolves_a_tabled_severity() {
    // C1: a Minor/Major/Critical wound with content takes EXACTLY ONE draw and resolves
    // the picked key to its InjuryDef.
    let (registry, tables) = one_injury_table("hurt", BodyPart::Torso, Severity::Major);
    let mut rng = injury_rng();
    let rolled = roll_injury(
        BodyPart::Torso,
        Severity::Major,
        &tables,
        &registry,
        &mut rng,
    );
    assert!(
        rolled.is_some(),
        "a populated Major bucket must roll an injury"
    );
    let Some(rolled) = rolled else {
        return;
    };
    assert_eq!(rolled.part, BodyPart::Torso);
    assert_eq!(rolled.severity, Severity::Major);
    assert_eq!(
        rolled.effects.len(),
        1,
        "the def's single effect is frozen on"
    );

    // EXACTLY ONE draw: the cursor advanced by one sample. A fresh stream that takes ONE
    // throwaway draw then matches this rng's next draw.
    let mut fresh = injury_rng();
    let _ = fresh.next_u64(); // one draw (the roll's)
    assert_eq!(
        rng.next_u64(),
        fresh.next_u64(),
        "a tabled roll must take EXACTLY ONE InjuryRng draw"
    );
}

#[test]
fn roll_injury_empty_table_draws_then_discards_content_independent() {
    // C1 + STREAM-ALIGNMENT (#2): the KEY content-independence property. A Minor wound on
    // an EMPTY/MISSING bucket STILL takes its one draw (then discards → None), so the
    // InjuryRng cursor ends at the SAME position whether the bucket has content or is
    // empty — a content hot-edit cannot desync replay.
    let empty_registry = InjuryRegistry::default();
    let empty_tables = InjuryTables::default();
    let (full_registry, full_tables) = one_injury_table("hurt", BodyPart::Head, Severity::Minor);

    // Drive BOTH scenarios from an identical fresh stream.
    let mut rng_empty = injury_rng();
    let empty = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        &empty_tables,
        &empty_registry,
        &mut rng_empty,
    );
    assert!(empty.is_none(), "an empty bucket rolls no injury");

    let mut rng_full = injury_rng();
    let full = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        &full_tables,
        &full_registry,
        &mut rng_full,
    );
    assert!(full.is_some(), "a populated bucket rolls an injury");

    // CONTENT-INDEPENDENCE: after one Minor roll each, BOTH cursors are at the SAME
    // position (each took EXACTLY ONE draw), regardless of whether content existed. The
    // post-draw streams produce identical next values.
    assert_eq!(
        rng_empty.next_u64(),
        rng_full.next_u64(),
        "the InjuryRng cursor must end at the SAME position for an empty vs a populated \
         bucket — the one draw is content-independent (stream alignment)"
    );
}

#[test]
fn roll_injury_is_deterministic_for_a_fixed_seed() {
    // C1: pure given the rng state — the same seed reproduces the same pick. (Determinism
    // at the roll level; the seeded-replay E2E lives in the acts injury test.)
    let (registry, tables) = one_injury_table("hurt", BodyPart::LeftLeg, Severity::Critical);
    let mut rng_a = injury_rng();
    let mut rng_b = injury_rng();
    let a = roll_injury(
        BodyPart::LeftLeg,
        Severity::Critical,
        &tables,
        &registry,
        &mut rng_a,
    );
    let b = roll_injury(
        BodyPart::LeftLeg,
        Severity::Critical,
        &tables,
        &registry,
        &mut rng_b,
    );
    assert_eq!(a, b, "the same seed must reproduce the same rolled injury");
}
