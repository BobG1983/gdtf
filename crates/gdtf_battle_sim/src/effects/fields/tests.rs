//! Palette-level behaviour suite for the GTW-553 field-consequence palette: the closed
//! [`FieldEffect`](super::FieldEffect) vocabulary parses from RON by variant name (the
//! serde bridge — pinned even though the vocabulary is not yet RON-exposed), the
//! [`FieldEffect::consequences_of`](super::FieldEffect::consequences_of) projection
//! mirrors an authored def exactly, and the enum's THIN delegation
//! `impl ApplyFieldEffect` routes each verb to its isolated behaviour.
//!
//! Per-consequence semantics are asserted in each consequence file's own `#[cfg(test)]`;
//! this suite proves the enum bridge (parse + projection + delegation), not the drain /
//! lifetime maths. Per the brittle-test rule, assertions check the MAPPING + DIRECTION,
//! never a shipped magnitude.

use super::{
    ApplyFieldEffect, FieldDamage, FieldDuration, FieldEffect, FieldTurns, ImmuneArmorTypes,
};
use crate::{armor::ArmorType, effects::fields::FieldDef, weapon::DamageType};

/// The closed [`FieldEffect`] vocabulary deserializes each variant from RON by name —
/// payload newtypes as bare scalars / lists (the `#[serde(transparent)]` bridge).
/// Pin-discriminating: a mis-named variant or a wrong payload shape fails to parse.
/// Asserts the MAPPING, not a magnitude.
#[test]
fn each_consequence_variant_parses_from_ron() {
    let Ok(drain) = ron::de::from_str::<FieldEffect>("Drain(damage: 3, damage_type: Chem)") else {
        unreachable!("Drain(damage:, damage_type:) must parse");
    };
    assert!(
        matches!(drain, FieldEffect::Drain { .. }),
        "Drain maps to the Drain variant"
    );

    let Ok(immunity) = ron::de::from_str::<FieldEffect>("Immunity(armor_types: [Flak])") else {
        unreachable!("Immunity(armor_types:) must parse");
    };
    assert!(
        matches!(immunity, FieldEffect::Immunity { .. }),
        "Immunity maps to the Immunity variant"
    );

    let Ok(turns) = ron::de::from_str::<FieldEffect>("Duration(Turns(2))") else {
        unreachable!("Duration(Turns(n)) must parse");
    };
    assert!(
        matches!(turns, FieldEffect::Duration(FieldDuration::Turns(_))),
        "Duration(Turns) maps to the finite lifetime"
    );

    let Ok(permanent) = ron::de::from_str::<FieldEffect>("Duration(Permanent)") else {
        unreachable!("Duration(Permanent) must parse");
    };
    assert!(
        matches!(permanent, FieldEffect::Duration(FieldDuration::Permanent)),
        "Duration(Permanent) maps to the unbounded lifetime"
    );
}

/// [`FieldEffect::consequences_of`] projects an authored [`FieldDef`] into EXACTLY its
/// three consequences, each carrying the def's own payload — the def→vocabulary bridge
/// the mechanics invoke generically (the authored RON surface stays the flat struct).
#[test]
fn consequences_of_mirrors_the_authored_def() {
    let def = FieldDef::new(
        FieldDamage::new(4),
        DamageType::Shock,
        ImmuneArmorTypes::new([ArmorType::Plated]),
        FieldDuration::Turns(FieldTurns::new(3)),
    );
    let consequences = FieldEffect::consequences_of(&def);
    assert_eq!(
        consequences,
        vec![
            FieldEffect::Drain {
                damage:      FieldDamage::new(4),
                damage_type: DamageType::Shock,
            },
            FieldEffect::Immunity {
                armor_types: ImmuneArmorTypes::new([ArmorType::Plated]),
            },
            FieldEffect::Duration(FieldDuration::Turns(FieldTurns::new(3))),
        ],
        "the projection carries each authored payload into its consequence variant"
    );
}

/// The [`FieldEffect`] enum's THIN delegation `impl ApplyFieldEffect` routes the lifetime
/// verbs to the isolated [`ApplyDuration`](super::ApplyDuration) — the enum's answers
/// match the isolated behaviour's for both seed and count-down. Proves the bridge
/// forwards, not that the enum carries logic (the drain / exemption delegation rides the
/// same `with_behaviour` match, exercised end-to-end by the `tick_fields` suite).
#[test]
fn the_enum_delegates_the_lifetime_verbs_to_the_isolated_behaviour() {
    let finite = FieldEffect::Duration(FieldDuration::Turns(FieldTurns::new(2)));
    let mut remaining = finite.initial_countdown();
    assert_eq!(*remaining, 2, "the enum seeds the authored Turns count");
    assert!(
        !finite.count_down_one_turn(&mut remaining),
        "2 → 1 through the enum: not yet expired"
    );
    assert!(
        finite.count_down_one_turn(&mut remaining),
        "1 → 0 through the enum: expired"
    );

    let permanent = FieldEffect::Duration(FieldDuration::Permanent);
    let mut forever = permanent.initial_countdown();
    assert!(
        !permanent.count_down_one_turn(&mut forever),
        "Permanent through the enum never expires"
    );

    // The non-lifetime consequences answer the lifetime verbs with the trait's inert
    // defaults — they neither seed nor expire.
    let drain = FieldEffect::Drain {
        damage:      FieldDamage::new(1),
        damage_type: DamageType::Chem,
    };
    assert_eq!(*drain.initial_countdown(), 0, "Drain seeds no countdown");
    let mut untouched = FieldTurns::new(5);
    assert!(
        !drain.count_down_one_turn(&mut untouched),
        "Drain never expires the field"
    );
    assert_eq!(*untouched, 5, "Drain never touches the countdown");
}
