//! RON argument and reply blobs, plus argument fault text.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// RON text of command arguments.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandArgsRon(String);

impl CommandArgsRon {
    /// Wrap a RON string.
    #[must_use]
    pub const fn new(ron: String) -> Self {
        Self(ron)
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// RON text of a successful command reply body.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandReplyRon(String);

impl CommandReplyRon {
    /// Wrap a RON string.
    #[must_use]
    pub const fn new(ron: String) -> Self {
        Self(ron)
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
