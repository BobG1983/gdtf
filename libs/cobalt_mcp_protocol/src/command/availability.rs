//! Whether a command can run, and why not if it cannot.

use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Availability of a catalogue entry or run attempt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandAvailability {
    /// Command may be run.
    Available,
    /// Command is refused in the current host state.
    Unavailable {
        /// Machine-readable code.
        code: UnavailableCode,
        /// Human note.
        note: RefusalNote,
    },
}

/// Why a command is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnavailableCode {
    /// Host is not in a state that allows this command.
    WrongState,
    /// A replay is in progress.
    Replaying,
    /// Required model data is missing.
    MissingModel,
    /// Feature was not built into this binary.
    NotBuilt,
}

impl UnavailableCode {
    /// All known codes.
    pub const ALL: [Self; 4] = [
        Self::WrongState,
        Self::Replaying,
        Self::MissingModel,
        Self::NotBuilt,
    ];
}

/// Human-readable refusal text.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RefusalNote(Cow<'static, str>);

impl RefusalNote {
    /// Borrow a static note.
    #[must_use]
    pub const fn from_static(note: &'static str) -> Self {
        Self(Cow::Borrowed(note))
    }

    /// Own a dynamic note.
    #[must_use]
    pub const fn from_owned(note: String) -> Self {
        Self(Cow::Owned(note))
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
