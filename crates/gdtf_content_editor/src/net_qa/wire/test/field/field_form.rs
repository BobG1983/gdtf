use gdtf_battle_sim::{effects::fields::FieldDamage, weapon::DamageType};

use super::{
    super::{assert_ron_round_trip, assert_schema_is_usable},
    support::a_name,
};
use crate::net_qa::wire::{
    DamageTypeNet, EditorFieldNet, FieldDamageNet, FieldDurationNet, FieldFormFieldNet,
    FieldTurnsNet,
};

#[test]
fn every_field_form_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::Field(FieldFormFieldNet::Name(a_name())));
    assert_ron_round_trip(&EditorFieldNet::Field(FieldFormFieldNet::Damage(
        FieldDamageNet::from_damage(FieldDamage::new(4)),
    )));
    for damage_type in DamageType::ALL {
        assert_ron_round_trip(&EditorFieldNet::Field(FieldFormFieldNet::DamageType(
            DamageTypeNet::from_damage_type(damage_type),
        )));
    }
    assert_ron_round_trip(&EditorFieldNet::Field(FieldFormFieldNet::Duration(
        FieldDurationNet::Permanent,
    )));
    assert_ron_round_trip(&EditorFieldNet::Field(FieldFormFieldNet::Duration(
        FieldDurationNet::Turns(FieldTurnsNet::new(3)),
    )));
    assert_ron_round_trip(&FieldDurationNet::Turns(FieldTurnsNet::new(0)));
}

#[test]
fn a_field_duration_in_turns_keeps_the_documented_wire_spelling() {
    let turns = EditorFieldNet::Field(FieldFormFieldNet::Duration(FieldDurationNet::Turns(
        FieldTurnsNet::new(3),
    )));
    let Ok(encoded) = ron::ser::to_string(&turns) else {
        unreachable!("a wire value serializes to compact RON: {turns:?}");
    };
    assert_eq!(
        encoded, "Field(Duration(Turns(3)))",
        "a client sends the field under the form that owns it, and the turn count as a plain \
         integer, so the newtype must stay transparent",
    );
}

#[test]
fn a_zero_turn_duration_crosses_the_wire_and_is_refused_by_the_reader() {
    assert_eq!(
        FieldDurationNet::Turns(FieldTurnsNet::new(0)).to_duration(),
        None,
        "the wire carries the turn count a client sent, and the reader is what refuses zero. \
         Clamping it to one here would hide the refusal the handler owes the client",
    );
    assert_eq!(
        FieldDurationNet::Turns(FieldTurnsNet::new(1))
            .to_duration()
            .map(FieldDurationNet::from_duration),
        Some(FieldDurationNet::Turns(FieldTurnsNet::new(1))),
        "one turn is the smallest count the sim loads, and it round-trips unchanged",
    );
}

#[test]
fn the_field_form_values_trace_usable_shapes() {
    assert_schema_is_usable::<FieldDamageNet>("FieldDamageNet");
    assert_schema_is_usable::<FieldDurationNet>("FieldDurationNet");
    assert_schema_is_usable::<FieldTurnsNet>("FieldTurnsNet");
}
