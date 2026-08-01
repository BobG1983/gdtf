//! The shared compact-RON round-trip assertion for this host's wire types (moved from
//! `gdtf_qa_protocol`'s own test support by GTW-943).

use core::fmt::Debug;

use serde::{Serialize, de::DeserializeOwned};

/// Assert `value` survives a compact-RON round-trip identically — the property every wire
/// type on this host owes (`ron::ser::to_string` → `ron::de::from_str` returns an equal
/// value).
///
/// Fails loudly (the house `let Ok(..) else { unreachable!() }` idiom, no bare
/// `panic!`/`unwrap`) if the value cannot encode, cannot decode, or decodes to a different
/// value — the failure message carries the compact-RON text so a regression is legible.
pub(in crate::dev::net_qa::wire) fn assert_ron_round_trip<T>(value: &T)
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
