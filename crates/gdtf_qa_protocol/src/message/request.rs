//! [`QaRequest`] — the client-to-server request vocabulary (GTW-734, cut to three by
//! GTW-943).

use serde::{Deserialize, Serialize};

use super::hello::ProtocolVersion;
use crate::command::{CommandArgsJson, CommandName, RunOptions};

/// A single request a QA client sends a host's `net_qa` server — one request in, one
/// [`QaResponse`](crate::message::QaResponse) out.
///
/// THREE variants, and that is the whole wire: open the session
/// ([`Hello`](Self::Hello)), read what this host offers ([`Catalogue`](Self::Catalogue)),
/// and run one of those things by name ([`Run`](Self::Run)).
///
/// A command is DATA, never a variant here. A host's whole vocabulary is carried inside the
/// catalogue reply and named by a string inside a `Run`, so adding a command to either host
/// moves neither this enum nor
/// [`ProtocolVersion::CURRENT`](crate::message::ProtocolVersion::CURRENT). That is what the
/// exhaustive `match` in `message/test/freeze.rs` guards: an author who has to edit this
/// enum has left the command layer and is changing the wire.
///
/// One enum serves BOTH hosts — the game's `net_qa` server and the editor's — so a QA
/// client speaks one vocabulary to either, and the difference between them is which
/// commands their catalogues list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaRequest {
    /// Open the session — negotiate the protocol version.
    ///
    /// Answered in the listener thread, before the host's inbox is reached (GTW-940): a
    /// match replies [`HelloOk`](crate::message::QaResponse::HelloOk), a mismatch
    /// [`VersionMismatch`](crate::message::QaError::VersionMismatch), and every other
    /// request on a connection that has not negotiated is refused
    /// [`NotNegotiated`](crate::message::QaError::NotNegotiated).
    Hello(ProtocolVersion),
    /// Ask the running host what commands it offers and which are available right now.
    ///
    /// The reply ([`Catalogue`](crate::message::QaResponse::Catalogue)) is a
    /// [`CommandCatalogue`](crate::command::CommandCatalogue): one row per command with its
    /// name, its one-line summary, when it answers, the derived JSON Schema of its
    /// arguments and of its reply, and whether it can run in the state the host is in right
    /// now.
    Catalogue,
    /// Run one command by name with its arguments.
    ///
    /// The reply ([`Outcome`](crate::message::QaResponse::Outcome)) is a
    /// [`CommandOutcome`](crate::command::CommandOutcome): what the command produced, why
    /// it was not admissible, which argument would not decode, or that no command of that
    /// name exists here.
    Run(RunCommand),
}

/// The body of a [`Run`](QaRequest::Run): which command, its arguments as JSON text, and
/// the per-call riders.
///
/// The arguments are opaque to this crate and to the MCP courier — only the command's own
/// `Args` type gives them meaning, and only the host that owns that command decodes them.
///
/// The [`options`](Self::options) riders are NOT opaque: they are the wire's own
/// vocabulary, read by the host's admission check rather than by any one command. Carrying
/// them here is what lets a host answer a call asking for machinery it has not built with
/// [`NotBuilt`](crate::command::UnavailableCode::NotBuilt) instead of running the command
/// and silently dropping the rider (GTW-942).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunCommand {
    /// The command to run, as its host advertised it in the catalogue.
    pub command:   CommandName,
    /// The command's arguments, as a JSON object.
    pub arguments: CommandArgsJson,
    /// How long to wait for admission, and whether to capture afterwards.
    ///
    /// `#[serde(default)]` so a frame encoded before this field existed still decodes, as
    /// [`RunOptions::default`] — both riders absent, which is what such a frame meant.
    #[serde(default)]
    pub options:   RunOptions,
}

impl RunCommand {
    /// Build a plain run request — a command name and its JSON arguments, no riders.
    #[must_use]
    pub fn new(command: CommandName, arguments: CommandArgsJson) -> Self {
        Self::with_options(command, arguments, RunOptions::default())
    }

    /// Build a run request carrying the per-call riders.
    #[must_use]
    pub const fn with_options(
        command: CommandName,
        arguments: CommandArgsJson,
        options: RunOptions,
    ) -> Self {
        Self {
            command,
            arguments,
            options,
        }
    }
}
