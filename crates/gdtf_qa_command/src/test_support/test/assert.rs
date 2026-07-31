//! Each conformance assertion FAILS on a deliberately broken set.
//!
//! An assertion nobody has watched fail is an assertion nobody knows works, so BOTH are
//! driven to a panic here, each over a real `&[&dyn ErasedCommand<FakeFacts>]` slice:
//! [`FAKE_COMMANDS_DUPLICATED`](crate::test_support::FAKE_COMMANDS_DUPLICATED) for the name
//! check, [`FAKE_COMMANDS_BROKEN_SCHEMA`](crate::test_support::FAKE_COMMANDS_BROKEN_SCHEMA)
//! for the schema check. The typed `check_*` layer underneath is exercised separately, on
//! hand-built rows, so a failure says which of the two layers broke.

use std::panic::{AssertUnwindSafe, catch_unwind};

use gdtf_qa_protocol::command::{ArgSchemaJson, CommandName, ReplySchemaJson};

use crate::{
    command::QaCommand,
    test_support::{
        CommandRow, FAKE_COMMANDS, FAKE_COMMANDS_BROKEN_SCHEMA, FAKE_COMMANDS_DUPLICATED,
        FakePhase, NameCheck, SchemaCheck, SchemaSide, assert_schemas_parse, assert_unique_names,
        check_schemas_parse, check_unique_names, command_rows,
    },
};

/// The good set passes both assertions.
#[test]
fn the_fake_set_passes_both_assertions() {
    assert_unique_names(FAKE_COMMANDS);
    assert_schemas_parse(FAKE_COMMANDS);
}

/// The uniqueness check names the duplicate, on a real slice.
#[test]
fn the_name_check_catches_a_duplicated_name() {
    let rows = command_rows(FAKE_COMMANDS_DUPLICATED);
    assert_eq!(
        check_unique_names(&rows),
        NameCheck::Duplicate(FakePhase::NAME)
    );
}

/// `assert_unique_names` itself fails on that slice — the check and the assertion are not
/// separately believable.
#[test]
fn the_name_assertion_fails_on_a_duplicated_name() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        assert_unique_names(FAKE_COMMANDS_DUPLICATED);
    }));
    assert!(
        outcome.is_err(),
        "assert_unique_names must fail on a set with two entries claiming one name"
    );
}

/// A row whose argument document is not JSON.
fn unparseable_argument_row() -> CommandRow {
    CommandRow {
        name:      CommandName::from_static("fake.broken"),
        arguments: ArgSchemaJson::new("{ this is not json".to_owned()),
        reply:     ReplySchemaJson::new("{}".to_owned()),
    }
}

/// A row whose reply document is not JSON.
fn unparseable_reply_row() -> CommandRow {
    CommandRow {
        name:      CommandName::from_static("fake.broken"),
        arguments: ArgSchemaJson::new("{}".to_owned()),
        reply:     ReplySchemaJson::new("<not json at all>".to_owned()),
    }
}

/// The schema check catches an unparseable ARGUMENT document and says which side failed.
#[test]
fn the_schema_check_catches_an_unparseable_argument_document() {
    assert_eq!(
        check_schemas_parse(&[unparseable_argument_row()]),
        SchemaCheck::Unparseable {
            command: CommandName::from_static("fake.broken"),
            which:   SchemaSide::Arguments,
        }
    );
}

/// The schema check catches an unparseable REPLY document too.
#[test]
fn the_schema_check_catches_an_unparseable_reply_document() {
    assert_eq!(
        check_schemas_parse(&[unparseable_reply_row()]),
        SchemaCheck::Unparseable {
            command: CommandName::from_static("fake.broken"),
            which:   SchemaSide::Reply,
        }
    );
}

/// The good rows still pass, so the two tests above are catching the defect and not the
/// checker being broken outright.
#[test]
fn the_schema_check_passes_the_real_rows() {
    assert_eq!(
        check_schemas_parse(&command_rows(FAKE_COMMANDS)),
        SchemaCheck::AllParse
    );
}

/// The schema check names the offender on a REAL slice, not only on a hand-built row.
#[test]
fn the_schema_check_catches_a_broken_slice_entry() {
    assert_eq!(
        check_schemas_parse(&command_rows(FAKE_COMMANDS_BROKEN_SCHEMA)),
        SchemaCheck::Unparseable {
            command: CommandName::from_static("fake.broken_schema"),
            which:   SchemaSide::Arguments,
        }
    );
}

/// `assert_schemas_parse` ITSELF fails on that slice — the check and the assertion are not
/// separately believable, and a host calls the assertion, never the check.
#[test]
fn the_schema_assertion_fails_on_a_broken_slice_entry() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        assert_schemas_parse(FAKE_COMMANDS_BROKEN_SCHEMA);
    }));
    assert!(
        outcome.is_err(),
        "assert_schemas_parse must fail on a set publishing a document that is not JSON"
    );
}
