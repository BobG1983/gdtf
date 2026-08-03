//! mirrors an authored def exactly, and the enum's THIN delegation
//! Per-consequence semantics are asserted in each consequence file's own `#[cfg(test)]`;
use super::{ApplyFieldEffect, FieldDamage, FieldDuration, FieldEffect, ImmuneArmorTypes};
use crate::{
    armor::ArmorType, effects::fields::FieldDef, test_support::field_turns, weapon::DamageType,
};

/// payload newtypes as bare scalars / lists (the `#[serde(transparent)]` bridge).
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

#[test]
fn consequences_of_mirrors_the_authored_def() {
    let def = FieldDef::new(
        FieldDamage::new(4),
        DamageType::Shock,
        ImmuneArmorTypes::new([ArmorType::Plated]),
        FieldDuration::Turns(field_turns(3)),
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
            FieldEffect::Duration(FieldDuration::Turns(field_turns(3))),
        ],
        "the projection carries each authored payload into its consequence variant"
    );
}

#[test]
fn the_enum_delegates_the_lifetime_verbs_to_the_isolated_behaviour() {
    let finite = FieldEffect::Duration(FieldDuration::Turns(field_turns(2)));
    let mut remaining = finite.initial_countdown();
    assert_eq!(
        remaining,
        Some(field_turns(2)),
        "the enum seeds the authored Turns count"
    );
    assert!(
        !*finite.count_down_one_turn(&mut remaining),
        "2 → 1 through the enum: not yet expired"
    );
    assert!(
        *finite.count_down_one_turn(&mut remaining),
        "1 → expired through the enum: the last round expires the placement"
    );

    let permanent = FieldEffect::Duration(FieldDuration::Permanent);
    let mut forever = permanent.initial_countdown();
    assert_eq!(
        forever, None,
        "Permanent through the enum carries no countdown"
    );
    assert!(
        !*permanent.count_down_one_turn(&mut forever),
        "Permanent through the enum never expires"
    );

    let drain = FieldEffect::Drain {
        damage:      FieldDamage::new(1),
        damage_type: DamageType::Chem,
    };
    assert_eq!(drain.initial_countdown(), None, "Drain seeds no countdown");
    let mut untouched = Some(field_turns(5));
    assert!(
        !*drain.count_down_one_turn(&mut untouched),
        "Drain never expires the field"
    );
    assert_eq!(
        untouched,
        Some(field_turns(5)),
        "Drain never touches the countdown"
    );
}
