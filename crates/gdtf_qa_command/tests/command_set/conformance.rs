use gdtf_qa_command::test_support::{
    FAKE_COMMANDS, FAKE_COMMANDS_GROWN, NameCheck, SchemaCheck, assert_schemas_parse,
    assert_unique_names, check_schemas_parse, check_unique_names, command_rows,
};

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
fn every_published_schema_is_a_json_object() {
    for row in command_rows(FAKE_COMMANDS_GROWN) {
        for (side, document) in [
            ("arguments", row.arguments.as_str()),
            ("reply", row.reply.as_str()),
        ] {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(document) else {
                unreachable!(
                    "{}'s {side} schema is not JSON: {document}",
                    row.name.as_str()
                );
            };
            assert!(
                value.is_object(),
                "{}'s {side} schema must be a JSON object: {document}",
                row.name.as_str()
            );
        }
    }
}
