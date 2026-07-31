//! The derived JSON Schema documents a catalogue row publishes — [`ArgSchemaJson`],
//! [`ReplySchemaJson`] (GTW-939).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The JSON Schema for a command's ARGUMENT type, as JSON text.
///
/// Always derived from the Rust type, never written by hand. A newtype over `String`
/// because the schema is carried as text — a JSON document inside a RON frame: converting
/// a JSON Schema through RON's data model and back is a lossy round trip nobody needs,
/// since the only consumer is an MCP client that wants JSON anyway. Private-inner
/// newtype (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgSchemaJson(String);

impl ArgSchemaJson {
    /// Build an argument schema from its derived JSON text.
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    /// This schema document as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The JSON Schema for a command's REPLY type, as JSON text.
///
/// Distinct from [`ArgSchemaJson`] (no-bare-types rule 3) — one says what to send, one
/// says what comes back, and the two must not be interchangeable at a call site. Carried
/// as text for the same reason. Serde-transparent.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReplySchemaJson(String);

impl ReplySchemaJson {
    /// Build a reply schema from its derived JSON text.
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    /// This schema document as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
