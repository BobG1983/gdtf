use gdtf_qa_protocol::command::{ShapeBody, ShapeDoc, shape_text};

use crate::{
    command::ErasedCommand,
    test_support::{FakeCell, FakeCellArgs, FakeCellReply, fake_facts_loaded},
};

fn parse(document: &str) -> ShapeDoc {
    let Ok(value) = ron::de::from_str::<ShapeDoc>(document) else {
        unreachable!("a traced shape must be valid RON, got: {document}");
    };
    value
}

fn record_fields(document: &str) -> Vec<String> {
    let doc = parse(document);
    let Some(ShapeBody::Record(fields)) = doc.root_body() else {
        unreachable!("the root of a named-field struct is a record: {document}");
    };
    fields
        .iter()
        .map(|field| field.name().as_str().to_owned())
        .collect()
}

#[test]
fn traced_argument_shape_names_every_field() {
    let document = shape_text::<FakeCellArgs>();
    assert!(
        record_fields(&document).iter().any(|field| field == "cell"),
        "the argument shape must name the `cell` field: {document}"
    );
}

#[test]
fn traced_reply_shape_names_every_field() {
    let document = shape_text::<FakeCellReply>();
    let fields = record_fields(&document);
    for wanted in ["cell", "level"] {
        assert!(
            fields.iter().any(|field| field == wanted),
            "the reply shape must name the `{wanted}` field: {document}"
        );
    }
}

#[test]
fn the_erased_view_publishes_the_traced_documents_unchanged() {
    let command: &dyn ErasedCommand<_> = &FakeCell;
    let _availability = command.availability(&fake_facts_loaded());

    assert_eq!(command.arg_schema().as_str(), shape_text::<FakeCellArgs>());
    assert_eq!(
        command.reply_schema().as_str(),
        shape_text::<FakeCellReply>()
    );
}
