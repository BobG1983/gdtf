//! Result of running a command (success, refusal, bad args, unknown).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::command::{
    ArgSchemaJson, ArgumentFault, CommandName, CommandReplyJson, RefusalNote, UnavailableCode,
};

/// Outcome of a run request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandOutcome {
    /// Command ran successfully.
    Ran {
        /// Reply body as JSON.
        reply: CommandReplyJson,
        /// Optional file attachments (e.g. PNG).
        attachments: Vec<ReplyAttachment>,
    },
    /// Command refused in the current host state.
    Unavailable {
        /// Machine-readable refusal code.
        code: UnavailableCode,
        /// Human note.
        note: RefusalNote,
    },
    /// Arguments failed validation.
    BadArguments {
        /// What was wrong.
        detail: ArgumentFault,
        /// Expected argument schema.
        schema: ArgSchemaJson,
    },
    /// Command name is not registered.
    Unknown {
        /// Names the host does know.
        known: Vec<CommandName>,
    },
}

/// File attached to a successful reply.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplyAttachment {
    /// Kind of artifact.
    pub kind: AttachmentKind,
    /// Path where the artifact was written.
    pub path: ArtifactPath,
}

impl ReplyAttachment {
    /// Build an attachment.
    #[must_use]
    pub const fn new(kind: AttachmentKind, path: ArtifactPath) -> Self {
        Self { kind, path }
    }
}

/// Supported attachment kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentKind {
    /// PNG image.
    Png,
}

impl AttachmentKind {
    /// All known kinds.
    pub const ALL: [Self; 1] = [Self::Png];
}

/// Filesystem path to an artifact.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactPath(String);

impl ArtifactPath {
    /// Wrap a path string.
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
