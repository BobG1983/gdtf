//! Assert unique command names and parseable RON shape documents.

use cobalt_mcp_protocol::command::{
    ArgSchemaRon, CommandName, ReplySchemaRon, ShapeBody, ShapeDoc, ShapeName,
};

use crate::command::ErasedCommand;

/// One command's name and shape texts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRow {
    /// Command name.
    pub name:      CommandName,
    /// Argument shape document.
    pub arguments: ArgSchemaRon,
    /// Reply shape document.
    pub reply:     ReplySchemaRon,
}

/// Result of a unique-name check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameCheck {
    /// All names are unique.
    Unique,
    /// First duplicated name found.
    Duplicate(CommandName),
}

/// Result of a shape-parse check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaCheck {
    /// Every document parses as a shape.
    AllParse,
    /// A document failed to parse.
    Unparseable {
        /// Command that published the bad document.
        command: CommandName,
        /// Which side failed.
        which:   SchemaSide,
    },
}

/// Which shape document failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaSide {
    /// Argument shape.
    Arguments,
    /// Reply shape.
    Reply,
}

/// Result of a shape-name agreement check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShapeNameCheck {
    /// Every type name carries one body across the whole set.
    Agree,
    /// A type name carries two different bodies.
    Disagree(ShapeName),
}

/// Collect name and shape rows from a command set.
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

/// Check that every shape document parses.
#[must_use]
pub fn check_schemas_parse(rows: &[CommandRow]) -> SchemaCheck {
    for row in rows {
        if ron::de::from_str::<ShapeDoc>(row.arguments.as_str()).is_err() {
            return SchemaCheck::Unparseable {
                command: row.name.clone(),
                which:   SchemaSide::Arguments,
            };
        }
        if ron::de::from_str::<ShapeDoc>(row.reply.as_str()).is_err() {
            return SchemaCheck::Unparseable {
                command: row.name.clone(),
                which:   SchemaSide::Reply,
            };
        }
    }
    SchemaCheck::AllParse
}

/// Check that one type name never carries two bodies across a command set.
#[must_use]
pub fn check_shape_names_agree(rows: &[CommandRow]) -> ShapeNameCheck {
    let mut seen: Vec<(ShapeName, ShapeBody)> = Vec::new();
    for row in rows {
        for document in [row.arguments.as_str(), row.reply.as_str()] {
            let Ok(doc) = ron::de::from_str::<ShapeDoc>(document) else {
                continue;
            };
            for def in doc.defs() {
                match seen.iter().find(|(name, _)| name == def.name()) {
                    Some((name, body)) if body != def.body() => {
                        return ShapeNameCheck::Disagree(name.clone());
                    }
                    Some(_) => {}
                    None => seen.push((def.name().clone(), def.body().clone())),
                }
            }
        }
    }
    ShapeNameCheck::Agree
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

/// Panic unless every published document is a shape.
///
/// # Panics
///
/// Panics when a document is not a RON shape.
pub fn assert_schemas_parse<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_schemas_parse(&rows),
        SchemaCheck::AllParse,
        "this host published a document that is not a RON shape"
    );
}

/// Panic unless every type name carries one body across the set.
///
/// # Panics
///
/// Panics when two types in one host's set share a name but not a body.
pub fn assert_shape_names_agree<F>(commands: &[&dyn ErasedCommand<F>]) {
    let rows = command_rows(commands);
    assert_eq!(
        check_shape_names_agree(&rows),
        ShapeNameCheck::Agree,
        "two types in this host's set publish one name with different bodies"
    );
}
