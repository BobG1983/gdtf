use super::assert_ron_round_trip;
use crate::dev::mcp::wire::{
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    offer::{ContextualActNet, ContextualOfferNet, OfferPressableNet, OfferTargetNet},
    token::{DoorToken, EmplacementToken, GangerToken},
};

const ACTS: [ContextualActNet; 8] = [
    ContextualActNet::Execute,
    ContextualActNet::Stabilize,
    ContextualActNet::Melee,
    ContextualActNet::Shove,
    ContextualActNet::OpenDoor,
    ContextualActNet::EnterEmplacement,
    ContextualActNet::ExitEmplacement,
    ContextualActNet::ThrowGrenade,
];

#[test]
fn every_offered_act_round_trips() {
    for act in ACTS {
        match act {
            ContextualActNet::Execute
            | ContextualActNet::Stabilize
            | ContextualActNet::Melee
            | ContextualActNet::Shove
            | ContextualActNet::OpenDoor
            | ContextualActNet::EnterEmplacement
            | ContextualActNet::ExitEmplacement
            | ContextualActNet::ThrowGrenade => {}
        }
        assert_ron_round_trip(&act);
    }
}

#[test]
fn every_offer_target_kind_round_trips() {
    let cell = CellLevelNet::new(
        CellNet::new(CellXNet::new(1), CellYNet::new(2)),
        LevelNet::new(0),
    );
    for target in [
        OfferTargetNet::Ganger(GangerToken::new(1)),
        OfferTargetNet::Door(DoorToken::new(2)),
        OfferTargetNet::Emplacement(EmplacementToken::new(3)),
        OfferTargetNet::Cell(cell),
    ] {
        match target {
            OfferTargetNet::Ganger(_)
            | OfferTargetNet::Door(_)
            | OfferTargetNet::Emplacement(_)
            | OfferTargetNet::Cell(_) => {}
        }
        assert_ron_round_trip(&target);
    }
}

#[test]
fn an_offer_round_trips() {
    let offer: ContextualOfferNet = ContextualOfferNet::new(
        ContextualActNet::Melee,
        OfferTargetNet::Ganger(GangerToken::new(9)),
        OfferPressableNet::new(true),
    );
    assert_ron_round_trip(&offer);
}

#[test]
fn a_greyed_out_offer_round_trips_as_not_pressable() {
    let offer = ContextualOfferNet::new(
        ContextualActNet::Shove,
        OfferTargetNet::Ganger(GangerToken::new(9)),
        OfferPressableNet::new(false),
    );
    assert!(
        !*offer.pressable,
        "an offer the actor cannot afford carries pressable = false",
    );
    assert_ron_round_trip(&offer);
}

#[test]
fn offered_acts_sort_in_their_declared_order() {
    let mut shuffled = vec![
        ContextualActNet::ThrowGrenade,
        ContextualActNet::Execute,
        ContextualActNet::Melee,
    ];
    shuffled.sort_unstable();
    assert_eq!(
        shuffled,
        vec![
            ContextualActNet::Execute,
            ContextualActNet::Melee,
            ContextualActNet::ThrowGrenade,
        ],
        "the offers list is sorted before it goes out, so a polling client sees one order",
    );
}
