use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::command::{
    ArgSchemaJson, ArgumentFault, CommandName, CommandReplyJson, RefusalNote, UnavailableCode,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandOutcome {
        Ran {
                reply:       CommandReplyJson,
                attachments: Vec<ReplyAttachment>,
    },
        Unavailable {
                code: UnavailableCode,
                note: RefusalNote,
    },
            BadArguments {
                detail: ArgumentFault,
                schema: ArgSchemaJson,
    },
            Unknown {
                known: Vec<CommandName>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplyAttachment {
        pub kind: AttachmentKind,
        pub path: ArtifactPath,
}

impl ReplyAttachment {
        #[must_use]
    pub const fn new(kind: AttachmentKind, path: ArtifactPath) -> Self {
        Self { kind, path }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentKind {
        Png,
}

impl AttachmentKind {
                    pub const ALL: [Self; 1] = [Self::Png];
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactPath(String);

impl ArtifactPath {
        #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
