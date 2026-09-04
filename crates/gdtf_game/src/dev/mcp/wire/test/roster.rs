use super::assert_ron_round_trip;
use crate::dev::mcp::wire::{
    act_payload::StanceNet,
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    roster::{FactionNet, GangerCardNet, GangerNameNet, MountedNet},
    token::GangerToken,
    vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
    wound::{BodyPartNet, InjuryNameNet, InjuryNet, SeverityNet, WoundNet},
};

pub(in crate::dev::mcp::wire::test) fn a_card() -> GangerCardNet {
    GangerCardNet {
        token:        GangerToken::new(4_294_967_296),
        at:           CellLevelNet::new(
            CellNet::new(CellXNet::new(7), CellYNet::new(-3)),
            LevelNet::new(1),
        ),
        name:         Some(GangerNameNet::new("Vex".to_owned())),
        faction:      FactionNet::new(0),
        stance:       StanceNet::Crouching,
        mounted:      MountedNet::new(true),
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
    assert_ron_round_trip(&MountedNet::new(true));
    assert_ron_round_trip(&MountedNet::new(false));
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
