use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a vitals value serializes to compact RON");
    };
    text
}

#[test]
fn vitals_scalars_round_trip() {
    assert_ron_round_trip(&TuNet::new(0));
    assert_ron_round_trip(&TuNet::new(u8::MAX));
    assert_ron_round_trip(&TuMaxNet::new(60));
    assert_ron_round_trip(&HpNet::new(0));
    assert_ron_round_trip(&HpNet::new(u16::MAX));
    assert_ron_round_trip(&HpMaxNet::new(12));
    assert_ron_round_trip(&WoundsNet::new(3));
    assert_ron_round_trip(&WoundsMaxNet::new(6));
}

#[test]
fn vitals_serialize_transparently() {
    assert_eq!(encoded(&TuNet::new(7)), "7");
    assert_eq!(encoded(&TuMaxNet::new(60)), "60");
    assert_eq!(encoded(&HpNet::new(9)), "9");
    assert_eq!(encoded(&HpMaxNet::new(12)), "12");
    assert_eq!(encoded(&WoundsNet::new(3)), "3");
    assert_eq!(encoded(&WoundsMaxNet::new(6)), "6");
}
