//! encoding produces. A round trip alone cannot see a lost `#[serde(transparent)]`: RON
use super::assert_ron_round_trip;
use crate::dev::mcp::wire::cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a coordinate serializes to compact RON");
    };
    text
}

#[test]
fn coordinate_newtypes_round_trip() {
    assert_ron_round_trip(&CellXNet::new(-3));
    assert_ron_round_trip(&CellYNet::new(58));
    assert_ron_round_trip(&LevelNet::new(4));
    assert_ron_round_trip(&CellNet::new(CellXNet::new(-1), CellYNet::new(2)));
    assert_ron_round_trip(&CellLevelNet::new(
        CellNet::new(CellXNet::new(5), CellYNet::new(6)),
        LevelNet::new(3),
    ));
}

#[test]
fn coordinate_scalars_serialize_transparently() {
    assert_eq!(encoded(&CellXNet::new(-3)), "-3");
    assert_eq!(encoded(&CellYNet::new(58)), "58");
    assert_eq!(encoded(&LevelNet::new(4)), "4");
}

#[test]
fn composed_keys_serialize_as_named_fields() {
    assert_eq!(
        encoded(&CellNet::new(CellXNet::new(1), CellYNet::new(2))),
        "(x:1,y:2)",
    );
    assert_eq!(
        encoded(&CellLevelNet::new(
            CellNet::new(CellXNet::new(1), CellYNet::new(2)),
            LevelNet::new(3),
        )),
        "(cell:(x:1,y:2),level:3)",
    );
}

#[test]
fn an_unknown_field_is_refused() {
    let hostile = "(cell:(x:1,y:2),level:3,storey:9)";
    assert!(
        ron::de::from_str::<CellLevelNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );

    let hostile = "(x:1,y:2,z:3)";
    assert!(
        ron::de::from_str::<CellNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}
