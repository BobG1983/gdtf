use core::fmt::Debug;

use gdtf_qa_protocol::command::{RonShape, ShapeDoc, shape_trace};
use serde::{Serialize, de::DeserializeOwned};

pub(in crate::net_qa::wire::test) fn assert_ron_round_trip<T>(value: &T)
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

fn named_root_resolves(doc: &ShapeDoc, named: &str) {
    let RonShape::Named(name) = doc.root() else {
        return;
    };
    assert!(
        doc.body_of(name).is_some(),
        "`{named}`'s shape references `{}` as its root but defines no body for it: {doc:?}",
        name.as_str(),
    );
}

pub(in crate::net_qa::wire::test) fn assert_schema_is_usable<T: DeserializeOwned>(named: &str) {
    match shape_trace::<T>() {
        Ok(doc) => named_root_resolves(&doc, named),
        Err(fault) => {
            unreachable!("`{named}`'s shape must trace out of its own Deserialize impl: {fault}")
        }
    }
}
