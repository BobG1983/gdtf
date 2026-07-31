//! The two conformance assertions each HOST runs over its own command slice, and the
//! typed checks underneath them.
//!
//! Two layers on purpose. The `assert_*` functions are what a host's suite calls; the
//! `check_*` functions return a typed verdict over the extracted rows, so this crate can
//! prove the detection works — including on an unparseable schema document, which no
//! `schemars`-derived command can actually produce.

use gdtf_qa_protocol::command::{ArgSchemaJson, CommandName, ReplySchemaJson};

use crate::command::ErasedCommand;

/// One command's published identity and its two derived schema documents.
///
/// The unit both checks work over. Extracting it first is what lets the checks be tested
/// against rows a real command could not produce.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRow {
    /// The command's name.
    pub name:      CommandName,
    /// Its derived argument schema.
    pub arguments: ArgSchemaJson,
    /// Its derived reply schema.
    pub reply:     ReplySchemaJson,
}

/// Whether a set's names are unique.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameCheck {
    /// Every name appears once.
    Unique,
    /// Two or more entries claim this name.
    Duplicate(CommandName),
}

/// Whether every published schema document parses as JSON.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaCheck {
    /// Every document parsed.
    AllParse,
    /// This command published a document that is not JSON.
    Unparseable {
        /// The command whose document failed.
        command: CommandName,
        /// Which of its two documents failed.
        which:   SchemaSide,
    },
}

/// Which of a command's two schema documents a check is talking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaSide {
    /// The argument schema.
    Arguments,
    /// The reply schema.
    Reply,
}

/// Extract one row per command, in slice order.
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

/// Whether these rows carry unique names.
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

/// Whether every document in these rows parses as JSON.
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

/// Assert that no two commands in this host's set share a name.
///
/// A host calls this from its own suite with its own slice — the assertion is a fact about
/// THAT list, and no test in this crate can see it. A duplicate name would make
/// [`admit()`](crate::dispatch::admit())'s linear scan resolve to whichever entry came first
/// and silently shadow the other.
///
/// # Panics
///
/// Fails the calling test when two entries in `commands` claim one name.
pub fn assert_unique_names<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_unique_names(&rows),
        NameCheck::Unique,
        "two commands in this host's set claim one name"
    );
}

/// Assert that every schema this host's set publishes is parseable JSON.
///
/// A catalogue row carries its two schemas as TEXT — a JSON document inside a RON frame —
/// so nothing on the wire would catch a document that is not JSON. This does.
///
/// # Panics
///
/// Fails the calling test when any published schema document is not parseable JSON.
pub fn assert_schemas_parse<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_schemas_parse(&rows),
        SchemaCheck::AllParse,
        "this host published a schema document that is not JSON"
    );
}
