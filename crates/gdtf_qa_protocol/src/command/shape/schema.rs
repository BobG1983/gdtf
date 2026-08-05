//! RON shape documents published for arguments and replies.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// RON shape describing command arguments.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgSchemaRon(String);

impl ArgSchemaRon {
    /// Wrap a shape document's RON text.
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

/// RON shape describing a successful reply body.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReplySchemaRon(String);

impl ReplySchemaRon {
    /// Wrap a shape document's RON text.
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
