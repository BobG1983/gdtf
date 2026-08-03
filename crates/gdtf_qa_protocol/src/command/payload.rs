//! JSON argument and reply blobs, plus argument fault text.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// JSON string of command arguments.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandArgsJson(String);

impl CommandArgsJson {
    /// Wrap a JSON string.
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// JSON string of a successful command reply body.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandReplyJson(String);

impl CommandReplyJson {
    /// Wrap a JSON string.
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why arguments were rejected.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgumentFault(String);

impl ArgumentFault {
    /// Wrap a detail string.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
