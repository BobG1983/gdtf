use gdtf_qa_protocol::command::{ArgSchemaJson, CommandName, ReplySchemaJson};

use crate::command::ErasedCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRow {
        pub name:      CommandName,
        pub arguments: ArgSchemaJson,
        pub reply:     ReplySchemaJson,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameCheck {
        Unique,
        Duplicate(CommandName),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaCheck {
        AllParse,
        Unparseable {
                command: CommandName,
                which:   SchemaSide,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaSide {
        Arguments,
        Reply,
}

#[must_use]
pub fn command_rows<F>(commands: &[&dyn ErasedCommand<F>]) -> Vec<CommandRow> {
    commands
        .iter()
        .map(|command| CommandRow {
            name:      command.name(),
            arguments: command.arg_schema(),
            reply:     command.reply_schema(),
        })
        .collect()
}

#[must_use]
pub fn check_unique_names(rows: &[CommandRow]) -> NameCheck {
    let mut seen: Vec<&CommandName> = Vec::with_capacity(rows.len());
    for row in rows {
        if seen.contains(&&row.name) {
            return NameCheck::Duplicate(row.name.clone());
        }
        seen.push(&row.name);
    }
    NameCheck::Unique
}

#[must_use]
pub fn check_schemas_parse(rows: &[CommandRow]) -> SchemaCheck {
    for row in rows {
        if serde_json::from_str::<serde_json::Value>(row.arguments.as_str()).is_err() {
            return SchemaCheck::Unparseable {
                command: row.name.clone(),
                which:   SchemaSide::Arguments,
            };
        }
        if serde_json::from_str::<serde_json::Value>(row.reply.as_str()).is_err() {
            return SchemaCheck::Unparseable {
                command: row.name.clone(),
                which:   SchemaSide::Reply,
            };
        }
    }
    SchemaCheck::AllParse
}

pub fn assert_unique_names<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_unique_names(&rows),
        NameCheck::Unique,
        "two commands in this host's set claim one name"
    );
}

pub fn assert_schemas_parse<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_schemas_parse(&rows),
        SchemaCheck::AllParse,
        "this host published a schema document that is not JSON"
    );
}
