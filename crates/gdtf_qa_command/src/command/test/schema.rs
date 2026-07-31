//! A derived schema document is valid JSON, names every field of its type, and closes the
//! record against unknown fields.

use crate::{
    command::{ErasedCommand, schema::schema_text},
    test_support::{FakeCell, FakeCellArgs, FakeCellReply, fake_facts_loaded},
};

/// Parse a schema document, failing the test loudly if it is not JSON.
fn parse(document: &str) -> serde_json::Value {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(document) else {
        unreachable!("a derived schema must be valid JSON, got: {document}");
    };
    value
}

/// The derived argument schema is JSON, names the type's field, and is closed.
#[test]
fn derived_argument_schema_names_every_field_and_is_closed() {
    let document = schema_text::<FakeCellArgs>();
    let value = parse(&document);

    assert!(
        value.pointer("/properties/cell").is_some(),
        "the argument schema must name the `cell` field: {document}"
    );
    assert_eq!(
        value.pointer("/additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "`deny_unknown_fields` must reach the published schema: {document}"
    );
}

/// The derived reply schema is JSON and names every field of the reply type.
#[test]
fn derived_reply_schema_names_every_field() {
    let document = schema_text::<FakeCellReply>();
    let value = parse(&document);

    for field in ["cell", "level"] {
        assert!(
            value.pointer(&format!("/properties/{field}")).is_some(),
            "the reply schema must name the `{field}` field: {document}"
        );
    }
}

/// The schemas a catalogue row publishes are the SAME documents the derivation produces —
/// the erased view adds nothing and rewrites nothing.
#[test]
fn the_erased_view_publishes_the_derived_documents_unchanged() {
    let command: &dyn ErasedCommand<_> = &FakeCell;
    // Touch the facts so the fixture is exercised the way a catalogue build would.
    let _availability = command.availability(&fake_facts_loaded());

    assert_eq!(command.arg_schema().as_str(), schema_text::<FakeCellArgs>());
    assert_eq!(
        command.reply_schema().as_str(),
        schema_text::<FakeCellReply>()
    );
}
