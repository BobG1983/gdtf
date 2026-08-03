//! Assert unique command names and parseable schema JSON.

use gdtf_qa_protocol::command::{ArgSchemaJson, CommandName, ReplySchemaJson};

use crate::command::ErasedCommand;

/// One command's name and schema texts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRow {
    /// Command name.
    pub name: CommandName,
    /// Argument schema JSON.
    pub arguments: ArgSchemaJson,
    /// Reply schema JSON.
    pub reply: ReplySchemaJson,
}

/// Result of a unique-name check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameCheck {
    /// All names are unique.
    Unique,
    /// First duplicated name found.
    Duplicate(CommandName),
}

/// Result of a schema-parse check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaCheck {
    /// Every schema parses as JSON.
    AllParse,
    /// A schema failed to parse.
    Unparseable {
        /// Command that published the bad schema.
        command: CommandName,
        /// Which side failed.
        which: SchemaSide,
    },
}

/// Which schema document failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaSide {
    /// Argument schema.
    Arguments,
    /// Reply schema.
    Reply,
}

/// Collect name and schema rows from a command set.
#[must_use]
pub fn command_rows<F>(commands: &[&dyn ErasedCommand<F>]) -> Vec<CommandRow> {
    commands
        .iter()
        .map(|command| CommandRow {
            name: command.name(),
            arguments: command.arg_schema(),
            reply: command.reply_schema(),
        })
        .collect()
}

/// Check that every command name appears once.
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

/// Check that every schema string is valid JSON.
#[must_use]
pub fn check_schemas_parse(rows: &[CommandRow]) -> SchemaCheck {
    for row in rows {
        if serde_json::from_str::<serde_json::Value>(row.arguments.as_str()).is_err() {
            return SchemaCheck::Unparseable {
                command: row.name.clone(),
                which: SchemaSide::Arguments,
            };
        }
        if serde_json::from_str::<serde_json::Value>(row.reply.as_str()).is_err() {
            return SchemaCheck::Unparseable {
                command: row.name.clone(),
                which: SchemaSide::Reply,
            };
        }
    }
    SchemaCheck::AllParse
}

/// Panic unless every command name is unique.
///
/// # Panics
///
/// Panics when two commands claim the same name.
pub fn assert_unique_names<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_unique_names(&rows),
        NameCheck::Unique,
        "two commands in this host's set claim one name"
    );
}

/// Panic unless every published schema is valid JSON.
///
/// # Panics
///
/// Panics when a schema document is not JSON.
pub fn assert_schemas_parse<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_schemas_parse(&rows),
        SchemaCheck::AllParse,
        "this host published a schema document that is not JSON"
    );
}
