//! Shared test helpers for the per-module round-trip suites (GTW-734).
//!
//! `pub(crate)` — the one home for the compact-RON round-trip assertion every module's
//! `test/` suite calls (module-layout rule 6: a helper with 2+ consumers lives in the
//! shared support module, not copied per file). Test-only (`#[cfg(test)]`).

use core::fmt::Debug;

use serde::{Serialize, de::DeserializeOwned};

/// Assert `value` survives a compact-RON round-trip identically — the ONE property the
/// whole protocol crate guarantees (`ron::ser::to_string` → `ron::de::from_str` returns
/// an equal value).
///
/// Fails loudly (the house `let Ok(..) else { unreachable!() }` idiom, no bare
/// `panic!`/`unwrap`) if the value cannot encode, cannot decode, or decodes to a
/// different value — the failure message carries the compact-RON text so a regression
/// is legible.
pub(crate) fn assert_ron_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let Ok(encoded) = ron::ser::to_string(value) else {
        unreachable!("a protocol value serializes to compact RON: {value:?}");
    };
    let Ok(decoded) = ron::de::from_str::<T>(&encoded) else {
        unreachable!("the compact RON `{encoded}` parses back to its type");
    };
    assert_eq!(
        &decoded, value,
        "the value changed across a compact-RON round-trip (via `{encoded}`)"
    );
}
