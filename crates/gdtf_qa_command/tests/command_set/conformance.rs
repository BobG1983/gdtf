//! The two shared assertions, called from ANOTHER crate's test with that crate's slice.
//!
//! This file is the proof of the clause: `tests/` is a separate compilation unit that links
//! `gdtf_qa_command` built without `--cfg test`, so calling the assertions here is exactly
//! what the game's and the editor's own suites will do with `GAME_COMMANDS` and
//! `EDITOR_COMMANDS`. The FAILURE side of each assertion is proved in-crate, in
//! `src/test_support/test/assert.rs`, where a deliberately broken set and an unparseable
//! schema document can be built.

use gdtf_qa_command::test_support::{
    FAKE_COMMANDS, FAKE_COMMANDS_GROWN, NameCheck, SchemaCheck, assert_schemas_parse,
    assert_unique_names, check_schemas_parse, check_unique_names, command_rows,
};

/// The fake host's set passes both assertions, called the way a host calls them.
#[test]
fn a_host_slice_passes_both_assertions() {
    assert_unique_names(FAKE_COMMANDS);
    assert_schemas_parse(FAKE_COMMANDS);
}

/// The grown set passes too — adding a command does not require touching the assertions.
#[test]
fn the_grown_slice_passes_both_assertions() {
    assert_unique_names(FAKE_COMMANDS_GROWN);
    assert_schemas_parse(FAKE_COMMANDS_GROWN);
}

/// The typed checks underneath are reachable from another crate as well, so a host can
/// report a violation rather than only fail on it.
#[test]
fn the_typed_checks_are_callable_from_another_crate() {
    let rows = command_rows(FAKE_COMMANDS_GROWN);
    assert_eq!(check_unique_names(&rows), NameCheck::Unique);
    assert_eq!(check_schemas_parse(&rows), SchemaCheck::AllParse);
}

/// Every published schema really is a JSON OBJECT, not merely parseable text.
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
