//! JSON schema blobs for arguments and replies.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// JSON schema describing command arguments.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgSchemaJson(String);

impl ArgSchemaJson {
    /// Wrap a schema JSON string.
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

/// JSON schema describing a successful reply body.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReplySchemaJson(String);

impl ReplySchemaJson {
    /// Wrap a schema JSON string.
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
