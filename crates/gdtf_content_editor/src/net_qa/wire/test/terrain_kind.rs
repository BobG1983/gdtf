use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{net_qa::wire::TerrainKindNet, terrain_form::TerrainKindChoice};

#[test]
fn every_terrain_kind_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(TerrainKindChoice::SEGMENT_ORDER.len());
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        let mirrored = format!("{:?}", TerrainKindNet::from_choice(choice));
        assert_eq!(
            mirrored,
            format!("{choice:?}"),
            "the wire mirror of {choice:?} must carry that kind's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{choice:?} maps onto {mirrored}, which another kind already claims — two kinds that \
             read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_terrain_kind_arm_round_trips() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_ron_round_trip(&TerrainKindNet::from_choice(choice));
    }
}

#[test]
fn every_terrain_kind_reads_back_as_the_pick_it_mirrored() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_eq!(
            TerrainKindNet::from_choice(choice).to_choice(),
            choice,
            "a client's kind must come back as the form's own, or the write would leave the \
             draft on a different kind than the one it was asked for",
        );
    }
}

#[test]
fn the_terrain_kind_traces_a_usable_shape() {
    assert_schema_is_usable::<TerrainKindNet>("TerrainKindNet");
}
