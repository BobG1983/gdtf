//! The live command catalogue a host publishes — [`CommandCatalogue`], [`CommandEntry`]
//! (GTW-939).

use serde::{Deserialize, Serialize};

use crate::{
    command::{
        ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming,
        ReplySchemaJson,
    },
    envelope::ServerNameNet,
};

/// The live command catalogue a host publishes — the reply to
/// [`Catalogue`](crate::envelope::QaRequest::Catalogue).
///
/// This is what [`AppFlowView::available`](crate::view::AppFlowView) could never be: that
/// listed request discriminants, so it could only say "you may send this kind". An entry
/// here says what the command is, what it takes, what it returns, and whether it can run
/// right now — read from the running app, which is the only authority an agent has.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandCatalogue {
    /// Which host answered — the same name the handshake reports.
    pub host:    ServerNameNet,
    /// Every command this host offers, in declaration order, available or not.
    pub entries: Vec<CommandEntry>,
}

impl CommandCatalogue {
    /// Build a catalogue from the answering host's name and its command rows.
    #[must_use]
    pub const fn new(host: ServerNameNet, entries: Vec<CommandEntry>) -> Self {
        Self { host, entries }
    }
}

/// One command's catalogue row.
///
/// The two schema fields carry DERIVED JSON Schema documents as text (see
/// [`ArgSchemaJson`]) — a JSON document inside a RON frame, which is the encoding this
/// design chose over converting schemas through RON's data model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandEntry {
    /// The name to run it by.
    pub command:      CommandName,
    /// One line saying what it does.
    pub summary:      CommandSummary,
    /// Whether the command answers on the claiming frame or on a later one.
    ///
    /// `#[serde(default)]` so a frame encoded before this field existed still decodes, as
    /// [`Immediate`](CommandTiming::Immediate) — the timing every command written before
    /// the field was added actually has.
    #[serde(default)]
    pub timing:       CommandTiming,
    /// The JSON Schema of its arguments, derived from the Rust type.
    pub arguments:    ArgSchemaJson,
    /// The JSON Schema of its reply, derived from the Rust type.
    pub reply:        ReplySchemaJson,
    /// Whether it can run in the state the host is in right now.
    pub availability: CommandAvailability,
}

impl CommandEntry {
    /// Build a catalogue row from a command's name, summary, declared timing, two derived
    /// schemas, and its availability at the moment the catalogue was read.
    #[must_use]
    pub const fn new(
        command: CommandName,
        summary: CommandSummary,
        timing: CommandTiming,
        arguments: ArgSchemaJson,
        reply: ReplySchemaJson,
        availability: CommandAvailability,
    ) -> Self {
        Self {
            command,
            summary,
            timing,
            arguments,
            reply,
            availability,
        }
    }
}
