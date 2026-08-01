//! What running a command produced — [`CommandOutcome`], [`ReplyAttachment`],
//! [`AttachmentKind`], [`ArtifactPath`] (GTW-939).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::command::{
    ArgSchemaJson, ArgumentFault, CommandName, CommandReplyJson, RefusalNote, UnavailableCode,
};

/// What running a command produced — the reply to
/// [`Run`](crate::message::QaRequest::Run).
///
/// FOUR variants, host-neutral. Note there is no `Refused`: a command that runs and says
/// no does so inside its OWN declared reply type, whose schema the catalogue publishes.
/// "The act was refused because the target is not adjacent" is domain vocabulary and
/// belongs to the command, not to the [`message`](crate::message) types that carry it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandOutcome {
    /// The command ran. `reply` is its declared reply type, as JSON.
    Ran {
        /// The command's reply body.
        reply:       CommandReplyJson,
        /// Files the host wrote that the caller should be handed — captures, exports.
        attachments: Vec<ReplyAttachment>,
    },
    /// The command exists but was not admissible in this state.
    Unavailable {
        /// The class of the refusal.
        code: UnavailableCode,
        /// The precondition, named.
        note: RefusalNote,
    },
    /// The arguments did not decode into the command's argument type. The schema rides
    /// along so a caller can fix the call without a second round trip.
    BadArguments {
        /// Which field was wrong and how, verbatim from the decoder.
        detail: ArgumentFault,
        /// The schema the arguments were checked against.
        schema: ArgSchemaJson,
    },
    /// No command of that name exists on this host. The known names ride along so a typo
    /// self-corrects in one round trip.
    Unknown {
        /// Every command name this host offers.
        known: Vec<CommandName>,
    },
}

/// A file a command produced that the caller should receive.
///
/// This is how a screenshot is meant to come back without the courier knowing what a
/// screenshot is: the courier will read the file (relative to the CHILD's working
/// directory — the GTW-923 resolution) and emit the right MCP content block for its
/// [`AttachmentKind`] — one generic rule, no per-command arm. `bins/gdtf_qa_mcp` does not
/// read attachments yet; this type is the shape that later work targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplyAttachment {
    /// What kind of file it is.
    pub kind: AttachmentKind,
    /// Where the host wrote it.
    pub path: ArtifactPath,
}

impl ReplyAttachment {
    /// Build an attachment from its kind and the path the host wrote it to.
    #[must_use]
    pub const fn new(kind: AttachmentKind, path: ArtifactPath) -> Self {
        Self { kind, path }
    }
}

/// The kinds of file a reply may attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentKind {
    /// A PNG image.
    Png,
}

impl AttachmentKind {
    /// Every attachment kind, in declaration order.
    ///
    /// The list the round-trip suite walks to prove each kind survives the wire; a new
    /// kind that is not listed here fails that test.
    pub const ALL: [Self; 1] = [Self::Png];
}

/// The path a host wrote an attachment to, as the host reported it.
///
/// Private-inner newtype over `String` (no-bare-types), serde-transparent. It is the
/// CHILD's path, meaningful only against the child's working directory — the courier
/// resolves it there, never against its own.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactPath(String);

impl ArtifactPath {
    /// Build an artifact path from the path text the host reported.
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }

    /// This path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
