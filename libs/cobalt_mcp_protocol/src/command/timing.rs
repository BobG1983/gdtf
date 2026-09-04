//! Whether a command finishes immediately or is deferred.

use serde::{Deserialize, Serialize};

/// Timing class for a catalogue entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandTiming {
    /// Completes in the same request/response.
    #[default]
    Immediate,
    /// Completes later (client may poll or wait).
    Deferred,
}

impl CommandTiming {
    /// All known timing classes.
    pub const ALL: [Self; 2] = [Self::Immediate, Self::Deferred];
}
