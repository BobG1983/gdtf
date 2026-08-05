use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    act_payload::StanceNet,
    roster::{FactionNet, GangerCardNet, GangerNameNet},
    token::GangerToken,
    vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
    wound::{BodyPartNet, InjuryNameNet, InjuryNet, SeverityNet, WoundNet},
};

pub(in crate::dev::net_qa::wire::test) fn a_card() -> GangerCardNet {
    GangerCardNet {
        token:        GangerToken::new(4_294_967_296),
        name:         Some(GangerNameNet::new("Vex".to_owned())),
        faction:      FactionNet::new(0),
        stance:       StanceNet::Crouching,
        tu:           TuNet::new(31),
        tu_max:       TuMaxNet::new(60),
        hp:           HpNet::new(8),
        hp_max:       HpMaxNet::new(12),
        wounds:       WoundsNet::new(2),
        wounds_max:   WoundsMaxNet::new(3),
        wounds_taken: vec![WoundNet::new(SeverityNet::Minor, BodyPartNet::RightArm)],
        injuries:     vec![InjuryNet::new(
            InjuryNameNet::new("cracked_rib".to_owned()),
            BodyPartNet::Torso,
            SeverityNet::Major,
        )],
    }
}

#[test]
fn roster_scalars_round_trip() {
    assert_ron_round_trip(&GangerNameNet::new("Vex".to_owned()));
    assert_ron_round_trip(&FactionNet::new(1));
}

#[test]
fn a_full_card_round_trips() {
    let card: GangerCardNet = a_card();
    assert_ron_round_trip(&card);
}

#[test]
fn a_nameless_card_with_empty_lists_round_trips() {
    let mut card: GangerCardNet = a_card();
    card.name = None;
    card.wounds_taken.clear();
    card.injuries.clear();
    assert_ron_round_trip(&card);
}

#[test]
fn a_card_refuses_an_unknown_field() {
    let hostile = "(token:1,mystery:2)";
    assert!(
        ron::de::from_str::<GangerCardNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}
