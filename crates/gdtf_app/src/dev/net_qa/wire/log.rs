//! Combat log entries, provenance and read limits on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::act_log::{ActEntry, ActProvenance};
use serde::{Deserialize, Serialize};

use super::{act::ActSeqNet, deed::ActDeedKindNet, token::GangerToken};

/// Why an act was logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActProvenanceNet {
    /// Player or QA commanded.
    Commanded,
    /// AI turn.
    AiTurn,
    /// Reaction fire interrupting another actor.
    Reaction {
        /// Actor that was interrupted.
        interrupted: GangerToken,
    },
    /// Clock / timer driven.
    Clock,
}

impl ActProvenanceNet {
    /// Mirror the sim's provenance.
    #[must_use]
    pub const fn from_sim(provenance: ActProvenance) -> Self {
        match provenance {
            ActProvenance::Commanded => Self::Commanded,
            ActProvenance::AiTurn => Self::AiTurn,
            ActProvenance::Reaction { interrupted } => Self::Reaction {
                interrupted: GangerToken::new(interrupted.to_bits()),
            },
            ActProvenance::Clock => Self::Clock,
        }
    }
}

/// One act-log line as a QA client reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogEntryNet {
    /// Monotonic sequence number.
    pub seq:        ActSeqNet,
    /// Who acted.
    pub actor:      GangerToken,
    /// Where the act came from.
    pub provenance: ActProvenanceNet,
    /// What kind of act it was.
    pub kind:       ActDeedKindNet,
}

impl LogEntryNet {
    /// Mirror a stored act-log entry.
    #[must_use]
    pub fn from_sim(entry: &ActEntry) -> Self {
        Self {
            seq:        ActSeqNet::new(*entry.seq()),
            actor:      GangerToken::new(entry.actor().to_bits()),
            provenance: ActProvenanceNet::from_sim(entry.provenance()),
            kind:       ActDeedKindNet::from_deed(entry.deed()),
        }
    }
}

/// Max number of log lines to return in one read.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogReadCap(u32);

impl LogReadCap {
    /// Build from a max line count.
    #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }
}

/// How many log lines were dropped before the read window.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogDroppedCount(u32);

impl LogDroppedCount {
    /// Build from a dropped count.
    #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }
}
