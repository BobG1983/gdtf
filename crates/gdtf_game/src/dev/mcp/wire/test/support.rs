use core::fmt::Debug;

use serde::{Serialize, de::DeserializeOwned};

pub(in crate::dev::mcp::wire) fn assert_ron_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let Ok(encoded) = ron::ser::to_string(value) else {
        unreachable!("a wire value serializes to compact RON: {value:?}");
    };
    let Ok(decoded) = ron::de::from_str::<T>(&encoded) else {
        unreachable!("the compact RON `{encoded}` parses back to its type");
    };
    assert_eq!(
        &decoded, value,
        "the value changed across a compact-RON round-trip (via `{encoded}`)"
    );
}
