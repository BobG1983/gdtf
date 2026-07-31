//! Round-trip + transparency pins for the id / handle newtypes (GTW-734).

use crate::{
    ids::{
        CellLevelNet, CellNet, CellXNet, CellYNet, DoorToken, EmplacementToken, EventCap,
        FireModeIndex, FrameDelay, GangerToken, LevelNet, RequestId, SeedNet, ShotName,
        SituationRef,
    },
    test_support::assert_ron_round_trip,
};

/// Every token / coordinate / handle newtype survives a compact-RON round-trip,
/// including negative cell coordinates (the floor-into-negative-cells case).
#[test]
fn id_newtypes_round_trip() {
    assert_ron_round_trip(&GangerToken::new(42));
    assert_ron_round_trip(&DoorToken::new(7));
    assert_ron_round_trip(&EmplacementToken::new(9));
    assert_ron_round_trip(&CellXNet::new(-3));
    assert_ron_round_trip(&CellYNet::new(58));
    assert_ron_round_trip(&LevelNet::new(4));
    assert_ron_round_trip(&CellNet::new(CellXNet::new(-1), CellYNet::new(2)));
    assert_ron_round_trip(&CellLevelNet::new(
        CellNet::new(CellXNet::new(5), CellYNet::new(6)),
        LevelNet::new(3),
    ));
    assert_ron_round_trip(&FireModeIndex::new(2));
    assert_ron_round_trip(&ShotName::new("aim_check".to_owned()));
    assert_ron_round_trip(&EventCap::new(64));
    assert_ron_round_trip(&SituationRef::new("skirmish".to_owned()));
    assert_ron_round_trip(&SeedNet::new(0xDEAD_BEEF));
    assert_ron_round_trip(&RequestId::new(1));
    assert_ron_round_trip(&FrameDelay::new(15));
}

/// A `#[serde(transparent)]` token rides the wire as its bare inner scalar, not a
/// wrapper tuple — the compact-wire guarantee.
#[test]
fn tokens_serialize_transparently() {
    let Ok(encoded) = ron::ser::to_string(&GangerToken::new(42)) else {
        unreachable!("a GangerToken serializes");
    };
    assert_eq!(encoded, "42", "the token is a bare scalar on the wire");
}

/// A `CellNet` rides the wire as a named-field pair of bare scalars.
#[test]
fn cell_serializes_as_named_pair() {
    let Ok(encoded) = ron::ser::to_string(&CellNet::new(CellXNet::new(1), CellYNet::new(2))) else {
        unreachable!("a CellNet serializes");
    };
    assert_eq!(
        encoded, "(x:1,y:2)",
        "the cell is an (x, y) pair on the wire"
    );
}
