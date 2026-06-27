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
    MovementCostFactor, PopupText, PostHeal, StatDelta, StatDeltaSum, StatKind, StatTarget,
    WeightedInjuryEntry, WeightedInjuryTable, roll_injury,
};
use crate::{
    armor::{BodyPart, InjuryCategory},
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
        category:     Arm,
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
    assert_eq!(def.category, InjuryCategory::Arm);
    assert!(matches!(def.effects[0], InjuryEffect::DisableHand));
    assert!(matches!(
        def.effects[1],
        InjuryEffect::Modify {
            stat: StatTarget::Shooting,
            ..
        }
    ));
}

// ── GTW-444: MovementCostMul multiplicative fold (C2 / C4 / C5 / C6) ──────────────

/// `f32` equality on a movement factor's inner — exact (the values under test are exactly
/// representable: 1.0, 1.5, 2.0, 3.0), so `==` is sound here.
fn factor_eq(a: MovementCostFactor, b: MovementCostFactor, msg: &str) {
    assert!(
        (a.raw() - b.raw()).abs() < f32::EPSILON,
        "{msg}: {a:?} != {b:?}"
    );
}

#[test]
fn movement_cost_factor_none_is_identity() {
    // C5 (none -> 1.0) + C6 identity: an empty ledger reports the IDENTITY factor (1.0),
    // so an uninjured mover is never accidentally slowed.
    let ledger = InflictedInjuries::default();
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::IDENTITY,
        "no injury -> identity (1.0)",
    );
}

#[test]
fn movement_cost_factor_one_mul_is_that_factor() {
    // C5 (one 1.5 -> 1.5): a single MovementCostMul(1.5) folds to exactly 1.5.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
    ));
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::new(1.5),
        "one MovementCostMul(1.5) -> 1.5",
    );
}

#[test]
fn movement_cost_factor_two_muls_multiply_not_add() {
    // C4 / C5 — THE SHARPEST PIN: two MovementCostMul (1.5, 2.0) MULTIPLY to 3.0, NOT add
    // to 3.5 and NOT sum to 3.5. An additive fold would FAIL this (it would give 3.5).
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
    ));
    ledger.gain(gained_on(
        BodyPart::RightLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(2.0))],
    ));
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::new(3.0),
        "two MovementCostMul (1.5, 2.0) MULTIPLY to 3.0 (NOT add to 3.5)",
    );
}

#[test]
fn movement_cost_factor_stacking_is_order_independent() {
    // C4: multiplication is commutative — gaining (2.0, 1.5) gives the SAME 3.0 as
    // (1.5, 2.0). Order-independent, deterministic.
    let mut a = InflictedInjuries::default();
    a.gain(gained_on(
        BodyPart::LeftLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(2.0))],
    ));
    a.gain(gained_on(
        BodyPart::RightLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
    ));
    factor_eq(
        a.movement_cost_factor(),
        MovementCostFactor::new(3.0),
        "stacking order does not matter (2.0 then 1.5 == 3.0)",
    );
}

#[test]
fn movement_cost_factor_other_effects_are_inert() {
    // C5 (a non-MovementCostMul effect -> 1.0): a Bleeding and a DisableHand (and a Modify)
    // contribute NOTHING to the movement factor — it stays IDENTITY. Pin-discriminating: a
    // fold that touched the movement accumulator for the wrong variant would FAIL.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    ledger.gain(gained_with(vec![InjuryEffect::Bleeding {
        amount: BleedAmount::new(2),
    }]));
    ledger.gain(gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Aim,
        amount: StatDelta::new(-2),
    }]));
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::IDENTITY,
        "Bleeding / DisableHand / Modify must leave the movement factor at identity",
    );
}

#[test]
fn movement_cost_mul_deserializes_from_ron() {
    // C1: the MovementCostMul(<f32>) variant parses from an effects Vec — the .injury.ron
    // shape the Hampered leg injuries use (the authored factor is a bare RON number).
    let ron = r#"(
        name:         "Shattered Knee",
        category:     Leg,
        severity:     Major,
        popup_text:   "KNEE SHATTERED",
        log_text:     "shatters a knee",
        inspect_text: "Shattered Knee -- Hampered movement",
        effects: [
            MovementCostMul(1.5),
            Modify(stat: Speed, amount: -1),
        ],
    )"#;
    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "the MovementCostMul effect must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.category, InjuryCategory::Leg);
    assert!(matches!(def.effects[0], InjuryEffect::MovementCostMul(_)));
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
        category:     Head,
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

    assert_eq!(def.category, InjuryCategory::Head);
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
        category:     Head,
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
        category: part.injury_category(),
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
        part.injury_category(),
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

// ── GTW-440: per-CATEGORY pool resolution (C1 / C5a) + side-from-part (C3 / C5b) ──

/// Build a single-entry catalog whose ONE injury key is authored into the ARM category
/// (its def's `category` is `Arm`) — so a roll keyed by EITHER arm must resolve this same
/// shared entry. The injury carries a `DisableHand` so the rolled verdict can be folded to
/// prove the side-from-part mapping.
fn shared_arm_table(key: &str) -> (InjuryRegistry, InjuryTables) {
    let name = InjuryName::new(key.to_owned());
    let def = InjuryDef {
        name:         name.clone(),
        // Authored into the shared Arm category (NOT a side); the rolled verdict's `part`
        // comes from the STRUCK side, not this field (GTW-440 C3 / GTW-453).
        category:     InjuryCategory::Arm,
        severity:     Severity::Major,
        popup_text:   PopupText::new("HAND SHATTERED".to_owned()),
        log_text:     LogText::new("shatters a hand".to_owned()),
        inspect_text: InspectText::new("Shattered Hand".to_owned()),
        effects:      vec![InjuryEffect::DisableHand],
        post_heal:    PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(name.clone(), def)]);
    let mut tables = InjuryTables::default();
    // Insert keyed by the shared Arm category bucket, so a lookup from EITHER arm side
    // (`LeftArm` / `RightArm`) resolves the SAME table.
    tables.insert(
        InjuryCategory::Arm,
        Severity::Major,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );
    (registry, tables)
}

#[test]
fn left_and_right_arm_sample_the_same_shared_arm_pool() {
    // C5(a) — THE STRUCTURAL PIN: a roll on LeftArm AND a roll on RightArm BOTH resolve the
    // ONE shared Arm-category pool (the table was inserted keyed by LeftArm only). Each
    // rolls the catalog's single `shattered_hand`. Pin-discriminating: were the tables
    // still keyed per-SIDE (no category collapse), the RightArm roll would find an empty /
    // missing bucket and return None — this test would FAIL.
    let (registry, tables) = shared_arm_table("shattered_hand");

    let mut rng_left = injury_rng();
    let left = roll_injury(
        BodyPart::LeftArm,
        Severity::Major,
        &tables,
        &registry,
        &mut rng_left,
    );
    let mut rng_right = injury_rng();
    let right = roll_injury(
        BodyPart::RightArm,
        Severity::Major,
        &tables,
        &registry,
        &mut rng_right,
    );

    assert!(
        left.is_some(),
        "a LeftArm Major roll must sample the shared Arm pool"
    );
    assert!(
        right.is_some(),
        "a RightArm Major roll must sample the SAME shared Arm pool (per-category resolution)",
    );
    // Both drew the same catalog entry (the one `shattered_hand` key).
    assert_eq!(
        left.as_ref().map(|r| r.name.clone()),
        Some(InjuryName::new("shattered_hand".to_owned())),
    );
    assert_eq!(
        right.as_ref().map(|r| r.name.clone()),
        left.as_ref().map(|r| r.name.clone()),
        "LeftArm and RightArm draw the identical shared-pool injury",
    );
}

#[test]
fn disable_hand_side_comes_from_struck_part_not_the_file() {
    // C5(b) — THE SIDE-FROM-PART PIN: a `DisableHand` rolled on LeftArm disables the LEFT
    // hand, the SAME def rolled on RightArm disables the RIGHT hand — even though BOTH draw
    // from the ONE shared Arm pool whose def authors `category: Arm` (side-agnostic). The
    // rolled verdict's `part` is the STRUCK side, so folding it disables the correct hand.
    // Pin-discriminating: if the side were taken from the def's authored `category` (Arm,
    // which has no side) instead of the struck part, neither roll could distinguish left
    // from right and the two ledgers below would be identical — this test would FAIL.
    let (registry, tables) = shared_arm_table("shattered_hand");

    // Roll on the LEFT arm, fold the verdict, then roll on the RIGHT arm and fold it onto a
    // SEPARATE ledger. Both rolls draw the same shared-pool def.
    let mut rng_left = injury_rng();
    let left_pick = roll_injury(
        BodyPart::LeftArm,
        Severity::Major,
        &tables,
        &registry,
        &mut rng_left,
    );
    let mut rng_right = injury_rng();
    let right_pick = roll_injury(
        BodyPart::RightArm,
        Severity::Major,
        &tables,
        &registry,
        &mut rng_right,
    );
    assert!(
        left_pick.is_some() && right_pick.is_some(),
        "the shared Arm pool must roll its catalog entry for BOTH arms",
    );
    let (Some(left_rolled), Some(right_rolled)) = (left_pick, right_pick) else {
        return;
    };

    // The rolled verdict records the STRUCK side, not the def's `category`.
    assert_eq!(
        left_rolled.part,
        BodyPart::LeftArm,
        "the rolled verdict's part is the struck LEFT arm",
    );
    assert_eq!(
        right_rolled.part,
        BodyPart::RightArm,
        "the rolled verdict's part is the struck RIGHT arm (not the def's side-agnostic Arm category)",
    );

    // Fold ONLY the left-arm injury onto one ledger: the LEFT hand is disabled, the right
    // stays (so the right-hand-only weapon still has a hand). With one hand disabled, two
    // remain minus one = one hand available.
    let mut left_only = InflictedInjuries::default();
    left_only.gain(left_rolled.into_gained());
    assert_eq!(
        left_only.hands_available(),
        HandsAvailable::new(1),
        "a LeftArm DisableHand disables exactly one (the left) hand",
    );

    // Fold BOTH the left-arm and the right-arm injuries: BOTH hands disabled → zero. This
    // can ONLY reach zero if the two injuries disabled DIFFERENT sides (a set over sides);
    // were the side taken from the shared def (both LeftArm), the set would hold one side
    // and one hand would remain — failing this assertion.
    let mut both = InflictedInjuries::default();
    both.gain(left_only.gained()[0].clone());
    both.gain(right_rolled.into_gained());
    assert_eq!(
        both.hands_available(),
        HandsAvailable::new(0),
        "a LeftArm + a RightArm DisableHand (both from the shared Arm pool) disable BOTH \
         hands — the side comes from the struck part, not the def",
    );
}

#[test]
fn leg_movement_cost_mul_yields_a_hampered_factor_above_one() {
    // C5(c): a MovementCostMul rolled on a LEG yields a Hampered movement factor > 1.0
    // (the ganger moves slower). Drives the real roll over a leg-category pool, folds the
    // verdict, and asserts the ledger's movement factor exceeds the identity 1.0.
    // Pin-discriminating: a factor of exactly 1.0 (identity) would mean the Hampered effect
    // never folded — this fails.
    let name = InjuryName::new("hampered_leg".to_owned());
    let def = InjuryDef {
        name:         name.clone(),
        category:     InjuryCategory::Leg,
        severity:     Severity::Major,
        popup_text:   PopupText::new("HAMPERED".to_owned()),
        log_text:     LogText::new("is hampered".to_owned()),
        inspect_text: InspectText::new("Hampered".to_owned()),
        effects:      vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
        post_heal:    PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(name.clone(), def)]);
    let mut tables = InjuryTables::default();
    tables.insert(
        InjuryCategory::Leg,
        Severity::Major,
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(name, InjuryWeight::new(10))]),
    );

    // A RightLeg roll resolves the SAME shared Leg pool (inserted keyed by LeftLeg).
    let mut rng = injury_rng();
    let pick = roll_injury(
        BodyPart::RightLeg,
        Severity::Major,
        &tables,
        &registry,
        &mut rng,
    );
    assert!(
        pick.is_some(),
        "the shared Leg pool must roll its catalog entry"
    );
    let Some(rolled) = pick else {
        return;
    };
    let mut ledger = InflictedInjuries::default();
    ledger.gain(rolled.into_gained());

    assert!(
        ledger.movement_cost_factor().raw() > 1.0,
        "a leg MovementCostMul must yield a Hampered factor > 1.0 (got {:?})",
        ledger.movement_cost_factor(),
    );
}
