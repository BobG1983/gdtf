use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    metric::{Cell, CellLevel, Level},
    terrain::def::TerrainUuid,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    connector_pairing::PairingOutcome,
    net_qa::wire::{PairingOutcomeNet, cell::EditorCellLevelNet},
};

fn a_painted_stair() -> TerrainUuid {
    TerrainUuid::new(Uuid::from_u128(0x51A1))
}

fn painted_slot() -> CellLevel {
    CellLevel::new(Cell::new(2, 3), Level::new(0))
}

fn slot_above() -> CellLevel {
    CellLevel::new(Cell::new(2, 3), Level::new(1))
}

// Every way a paint can end, so no arm can be dropped unnoticed.
fn every_outcome() -> [PairingOutcome; 4] {
    [
        PairingOutcome::Rejected,
        PairingOutcome::PlacedNoPair,
        PairingOutcome::PlacedPairSkipped,
        PairingOutcome::PairPlaced {
            paired: a_painted_stair(),
            at:     slot_above(),
        },
    ]
}

#[test]
fn every_pairing_outcome_round_trips() {
    for outcome in every_outcome() {
        assert_ron_round_trip(&PairingOutcomeNet::from_outcome(outcome));
    }
}

#[test]
fn a_placed_pair_names_the_paired_key_and_the_slot_it_landed_in() {
    let mirrored = PairingOutcomeNet::from_outcome(PairingOutcome::PairPlaced {
        paired: a_painted_stair(),
        at:     slot_above(),
    });
    let PairingOutcomeNet::PairPlaced { paired, at } = &mirrored else {
        unreachable!("built as a PairPlaced above, got {mirrored:?}");
    };
    assert_eq!(
        **paired,
        (*a_painted_stair()).to_string(),
        "the reply names the tile the pass placed one storey up, so a caller knows which key \
         appeared in a cell it never asked for",
    );
    assert_ne!(
        *at,
        EditorCellLevelNet::from_slot(painted_slot()),
        "the pair lands one storey above the painted slot, so a reply carrying the painted \
         storey would send a client to the wrong level: {mirrored:?}",
    );
    assert_eq!(
        *at,
        EditorCellLevelNet::from_slot(slot_above()),
        "the reply names the slot the second tile landed in: {mirrored:?}",
    );
}

#[test]
fn every_outcome_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::new();
    for outcome in every_outcome() {
        let mirrored = format!("{:?}", PairingOutcomeNet::from_outcome(outcome));
        assert!(
            !seen.contains(&mirrored),
            "{outcome:?} maps onto {mirrored}, which another outcome already claims. Two \
             results that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn the_pairing_outcome_traces_a_usable_shape() {
    assert_schema_is_usable::<PairingOutcomeNet>("PairingOutcomeNet");
}
