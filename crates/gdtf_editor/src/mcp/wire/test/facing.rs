use gdtf_battle_sim::terrain::facing::TerrainFacing;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::mcp::wire::{TerrainCornerNet, TerrainFacingNet};

#[test]
fn every_facing_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(TerrainFacing::ALL.len());
    for facing in TerrainFacing::ALL {
        let mirrored = format!("{:?}", TerrainFacingNet::from_facing(facing));
        assert_eq!(
            mirrored,
            format!("{facing:?}"),
            "the wire mirror of {facing:?} must carry that cardinal's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{facing:?} maps onto {mirrored}, which another cardinal already claims — two sides \
             that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_facing_arm_round_trips() {
    for facing in TerrainFacing::ALL {
        assert_ron_round_trip(&TerrainFacingNet::from_facing(facing));
    }
}

#[test]
fn every_facing_reads_back_as_the_cardinal_it_mirrored() {
    for facing in TerrainFacing::ALL {
        assert_eq!(
            TerrainFacingNet::from_facing(facing).to_facing(),
            facing,
            "a client's side must come back as the sim's own, or a toggle would flip a different \
             side than the one it was asked for",
        );
    }
}

#[test]
fn every_corner_arm_round_trips() {
    for corner in TerrainCornerNet::ALL {
        assert_ron_round_trip(&corner);
    }
}

#[test]
fn every_corner_reads_back_as_the_corner_it_mirrored() {
    for corner in TerrainCornerNet::ALL {
        assert_eq!(
            TerrainCornerNet::from_corner(corner.to_corner()),
            corner,
            "a client's corner must come back as the sim's own, or a view written at one turn \
             would be stored at another",
        );
    }
}

#[test]
fn the_facing_traces_a_usable_shape() {
    assert_schema_is_usable::<TerrainFacingNet>("TerrainFacingNet");
    assert_schema_is_usable::<TerrainCornerNet>("TerrainCornerNet");
}
