use serde::{Deserialize, Serialize};

use crate::command::shape::{FALLBACK_SHAPE_TEXT, shape_text, shape_trace};

#[derive(Debug, Serialize, Deserialize)]
struct SelfReferential {
    next: Option<Box<Self>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ProbeUntagged {
    Count(u32),
    Label(String),
}

#[test]
fn a_self_referential_type_is_named_rather_than_overflowing_the_stack() {
    let Err(fault) = shape_trace::<SelfReferential>() else {
        unreachable!("a type that contains itself cannot be traced");
    };
    let reported = fault.to_string();
    assert!(
        reported.contains("SelfReferential"),
        "the fault names the offending type: {reported}",
    );
    assert_eq!(
        shape_text::<SelfReferential>(),
        FALLBACK_SHAPE_TEXT,
        "a failed trace publishes the fallback rather than a wrong document",
    );
}

#[test]
fn an_untagged_enum_is_named_rather_than_publishing_an_empty_shape() {
    let Err(fault) = shape_trace::<ProbeUntagged>() else {
        unreachable!("an untagged enum describes nothing to the deserializer");
    };
    let reported = fault.to_string();
    assert!(
        reported.contains("ProbeUntagged"),
        "the fault names the offending type: {reported}",
    );
    assert!(
        reported.contains("deserialize_any"),
        "the fault says what the type asked for: {reported}",
    );
    assert_eq!(shape_text::<ProbeUntagged>(), FALLBACK_SHAPE_TEXT);
}
