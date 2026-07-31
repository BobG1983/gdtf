//! The JSON bodies a command call carries — [`CommandArgsJson`], [`CommandReplyJson`],
//! [`ArgumentFault`] (GTW-939).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A command call's **arguments**, as JSON object text.
///
/// Opaque to the envelope and to the MCP courier: only the command's own `Args` type gives
/// them meaning, and only the host that owns that command decodes them. Private-inner
/// newtype over `String` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandArgsJson(String);

impl CommandArgsJson {
    /// Build an argument body from its JSON text.
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    /// This argument body as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A command's **reply body**, as JSON text produced from that command's declared reply
/// type.
///
/// A distinct newtype from [`CommandArgsJson`] (no-bare-types rule 3): one travels in, one
/// travels out, and swapping them at a call site must not compile. Serde-transparent.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandReplyJson(String);

impl CommandReplyJson {
    /// Build a reply body from its JSON text.
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    /// This reply body as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a [`CommandArgsJson`] would not decode into the command's argument type — the serde
/// error text, verbatim, so a caller sees which field was wrong.
///
/// Private-inner newtype over `String` (no-bare-types), serde-transparent. Carried by
/// [`CommandOutcome::BadArguments`](crate::command::CommandOutcome::BadArguments) beside
/// the schema the arguments were checked against, so one round trip is enough to fix the
/// call.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgumentFault(String);

impl ArgumentFault {
    /// Build an argument fault from the decoder's error text.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }

    /// This fault's text as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
