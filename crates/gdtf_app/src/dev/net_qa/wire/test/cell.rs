//! Round-trip + wire-text pins for the grid-coordinate types (GTW-944).
//!
//! Each scalar is pinned twice — once for the round trip, once for the compact TEXT its
//! encoding produces. A round trip alone cannot see a lost `#[serde(transparent)]`: RON
//! round-trips a non-transparent newtype perfectly well, it just writes `(4)` where the
//! wire contract says `4`.

use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet};

/// The compact RON `value` encodes to.
///
/// Fails loudly (the house `let Ok(..) else { unreachable!() }` idiom) if it cannot encode.
fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a coordinate serializes to compact RON");
    };
    text
}

/// Every coordinate newtype survives a compact-RON round trip, including a NEGATIVE cell
/// coordinate — the sim floors negative positions into negative cells, so that case is real.
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

/// Each coordinate scalar rides the wire as its BARE inner value, not a wrapper tuple.
#[test]
fn coordinate_scalars_serialize_transparently() {
    assert_eq!(encoded(&CellXNet::new(-3)), "-3");
    assert_eq!(encoded(&CellYNet::new(58)), "58");
    assert_eq!(encoded(&LevelNet::new(4)), "4");
}

/// A [`CellNet`] rides the wire as a named-field pair, and a [`CellLevelNet`] as that pair
/// beside its storey — the composed shapes, not loose numbers.
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

/// An unknown field is REFUSED rather than ignored — the `deny_unknown_fields` half of the
/// `additionalProperties: false` the derived schema publishes. Without this, a client that
/// misspells `level` gets a silent decode of the wrong shape.
#[test]
fn an_unknown_field_is_refused() {
    let hostile = "(cell:(x:1,y:2),level:3,storey:9)";
    assert!(
        ron::de::from_str::<CellLevelNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}
