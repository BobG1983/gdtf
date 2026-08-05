use std::panic::{AssertUnwindSafe, catch_unwind};

use gdtf_qa_protocol::command::{ArgSchemaRon, CommandName, ReplySchemaRon};

use crate::{
    command::QaCommand,
    test_support::{
        CommandRow, FAKE_COMMANDS, FAKE_COMMANDS_BROKEN_SCHEMA, FAKE_COMMANDS_DUPLICATED,
        FAKE_COMMANDS_GROWN, FakePhase, NameCheck, SchemaCheck, SchemaSide, ShapeNameCheck,
        assert_schemas_parse, assert_unique_names, check_schemas_parse, check_shape_names_agree,
        check_unique_names, command_rows,
    },
};

const EMPTY_SHAPE: &str = "(root:Unit,defs:[])";

#[test]
fn the_fake_set_passes_both_assertions() {
    assert_unique_names(FAKE_COMMANDS);
    assert_schemas_parse(FAKE_COMMANDS);
}

#[test]
fn the_name_check_catches_a_duplicated_name() {
    let rows = command_rows(FAKE_COMMANDS_DUPLICATED);
    assert_eq!(
        check_unique_names(&rows),
        NameCheck::Duplicate(FakePhase::NAME)
    );
}

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

fn unparseable_argument_row() -> CommandRow {
    CommandRow {
        name:      CommandName::from_static("fake.broken"),
        arguments: ArgSchemaRon::new("{ this is not a shape".to_owned()),
        reply:     ReplySchemaRon::new(EMPTY_SHAPE.to_owned()),
    }
}

fn unparseable_reply_row() -> CommandRow {
    CommandRow {
        name:      CommandName::from_static("fake.broken"),
        arguments: ArgSchemaRon::new(EMPTY_SHAPE.to_owned()),
        reply:     ReplySchemaRon::new("<not a shape at all>".to_owned()),
    }
}

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

#[test]
fn the_schema_check_passes_the_real_rows() {
    assert_eq!(
        check_schemas_parse(&command_rows(FAKE_COMMANDS)),
        SchemaCheck::AllParse
    );
}

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

#[test]
fn the_schema_assertion_fails_on_a_broken_slice_entry() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        assert_schemas_parse(FAKE_COMMANDS_BROKEN_SCHEMA);
    }));
    assert!(
        outcome.is_err(),
        "assert_schemas_parse must fail on a set publishing a document that is not a shape"
    );
}

#[test]
fn the_grown_set_publishes_one_body_per_type_name() {
    assert_eq!(
        check_shape_names_agree(&command_rows(FAKE_COMMANDS_GROWN)),
        ShapeNameCheck::Agree
    );
}

#[test]
fn the_shape_name_check_catches_two_bodies_under_one_name() {
    let rows = vec![
        CommandRow {
            name:      CommandName::from_static("fake.one"),
            arguments: ArgSchemaRon::new(
                r#"(root:Named("Clash"),defs:[("Clash",Record([("x",Int)]))])"#.to_owned(),
            ),
            reply:     ReplySchemaRon::new(EMPTY_SHAPE.to_owned()),
        },
        CommandRow {
            name:      CommandName::from_static("fake.two"),
            arguments: ArgSchemaRon::new(
                r#"(root:Named("Clash"),defs:[("Clash",Record([("y",Text)]))])"#.to_owned(),
            ),
            reply:     ReplySchemaRon::new(EMPTY_SHAPE.to_owned()),
        },
    ];
    assert_eq!(
        check_shape_names_agree(&rows),
        ShapeNameCheck::Disagree(gdtf_qa_protocol::command::ShapeName::from_static("Clash"))
    );
}
