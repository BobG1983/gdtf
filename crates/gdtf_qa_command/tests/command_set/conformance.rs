use gdtf_qa_command::test_support::{
    FAKE_COMMANDS, FAKE_COMMANDS_BROKEN_SCHEMA, FAKE_COMMANDS_GROWN, NameCheck, SchemaCheck,
    SchemaSide, ShapeNameCheck, assert_schemas_parse, assert_shape_names_agree,
    assert_unique_names, check_schemas_parse, check_shape_names_agree, check_unique_names,
    command_rows,
};
use gdtf_qa_protocol::command::{CommandName, ShapeDoc};

#[test]
fn a_host_slice_passes_both_assertions() {
    assert_unique_names(FAKE_COMMANDS);
    assert_schemas_parse(FAKE_COMMANDS);
}

#[test]
fn the_grown_slice_passes_both_assertions() {
    assert_unique_names(FAKE_COMMANDS_GROWN);
    assert_schemas_parse(FAKE_COMMANDS_GROWN);
}

#[test]
fn the_typed_checks_are_callable_from_another_crate() {
    let rows = command_rows(FAKE_COMMANDS_GROWN);
    assert_eq!(check_unique_names(&rows), NameCheck::Unique);
    assert_eq!(check_schemas_parse(&rows), SchemaCheck::AllParse);
}

#[test]
fn every_published_schema_is_a_shape_document() {
    for row in command_rows(FAKE_COMMANDS_GROWN) {
        for (side, document) in [
            ("arguments", row.arguments.as_str()),
            ("reply", row.reply.as_str()),
        ] {
            let Ok(doc) = ron::de::from_str::<ShapeDoc>(document) else {
                unreachable!(
                    "{}'s {side} shape is not RON: {document}",
                    row.name.as_str()
                );
            };
            assert!(
                doc.root_body().is_some(),
                "{}'s {side} root must resolve to a definition: {document}",
                row.name.as_str()
            );
        }
    }
}

#[test]
fn a_broken_document_still_fails_the_shape_check() {
    assert_eq!(
        check_schemas_parse(&command_rows(FAKE_COMMANDS_BROKEN_SCHEMA)),
        SchemaCheck::Unparseable {
            command: CommandName::from_static("fake.broken_schema"),
            which:   SchemaSide::Arguments,
        }
    );
}

#[test]
fn every_type_name_in_the_grown_slice_carries_one_body() {
    assert_eq!(
        check_shape_names_agree(&command_rows(FAKE_COMMANDS_GROWN)),
        ShapeNameCheck::Agree
    );
    assert_shape_names_agree(FAKE_COMMANDS_GROWN);
}
