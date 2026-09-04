use crate::{ids::ShotName, test_support::assert_ron_round_trip};

#[test]
fn id_newtypes_round_trip() {
    assert_ron_round_trip(&ShotName::new("aim_check".to_owned()));
}

/// A `#[serde(transparent)]` newtype rides the wire as its bare inner scalar, not a
#[test]
fn scalars_serialize_transparently() {
    let Ok(encoded) = ron::ser::to_string(&ShotName::new("aim_check".to_owned())) else {
        unreachable!("a ShotName serializes");
    };
    assert_eq!(
        encoded, "\"aim_check\"",
        "the shot name is a bare string on the wire"
    );
}
