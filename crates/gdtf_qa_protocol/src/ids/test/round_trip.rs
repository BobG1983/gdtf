//! Round-trip + transparency pins for the surviving id newtypes (GTW-734).

use crate::{
    ids::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet, ShotName},
    test_support::assert_ron_round_trip,
};

/// Every coordinate / name newtype survives a compact-RON round-trip, including negative
/// cell coordinates (the floor-into-negative-cells case).
#[test]
fn id_newtypes_round_trip() {
    assert_ron_round_trip(&CellXNet::new(-3));
    assert_ron_round_trip(&CellYNet::new(58));
    assert_ron_round_trip(&LevelNet::new(4));
    assert_ron_round_trip(&CellNet::new(CellXNet::new(-1), CellYNet::new(2)));
    assert_ron_round_trip(&CellLevelNet::new(
        CellNet::new(CellXNet::new(5), CellYNet::new(6)),
        LevelNet::new(3),
    ));
    assert_ron_round_trip(&ShotName::new("aim_check".to_owned()));
}

/// A `#[serde(transparent)]` newtype rides the wire as its bare inner scalar, not a
/// wrapper tuple — the compact-wire guarantee.
#[test]
fn scalars_serialize_transparently() {
    let Ok(encoded) = ron::ser::to_string(&LevelNet::new(4)) else {
        unreachable!("a LevelNet serializes");
    };
    assert_eq!(encoded, "4", "the level is a bare scalar on the wire");
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
