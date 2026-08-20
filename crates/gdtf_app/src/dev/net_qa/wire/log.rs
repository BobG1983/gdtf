//! Combat log entries, provenance and read limits on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    act_log::{ActEntry, ActProvenance},
    ganger::Faction,
    visibility::{ActVisibility, ActorIdentified},
};
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
        /// Actor that was interrupted, absent when the asking gang could not identify it.
        interrupted: Option<GangerToken>,
    },
    /// Clock / timer driven.
    Clock,
}

impl ActProvenanceNet {
    /// Mirror the sim's provenance, naming the interrupted actor only when it was identified.
    #[must_use]
    pub fn from_sim(provenance: ActProvenance, identified: ActorIdentified) -> Self {
        match provenance {
            ActProvenance::Commanded => Self::Commanded,
            ActProvenance::AiTurn => Self::AiTurn,
            ActProvenance::Reaction { interrupted } => Self::Reaction {
                interrupted: token_if(interrupted, identified),
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
    /// Who acted, absent when the asking gang could not identify them.
    pub actor:      Option<GangerToken>,
    /// Where the act came from.
    pub provenance: ActProvenanceNet,
    /// What kind of act it was.
    pub kind:       ActDeedKindNet,
}

impl LogEntryNet {
    /// Mirror a stored entry as far as `view` allows, or nothing when the act is withheld.
    /// `view` decides the drop and the name, so a read cannot disagree with the panel.
    #[must_use]
    pub fn from_sim(entry: &ActEntry, asking: Faction, view: ActVisibility) -> Option<Self> {
        let names_actor = match view {
            ActVisibility::Withheld => return None,
            ActVisibility::Named => ActorIdentified::new(true),
            ActVisibility::Unnamed => ActorIdentified::new(false),
        };
        Some(Self::mirror(
            entry,
            names_actor,
            entry.witnesses().identifies_interrupted(asking),
        ))
    }

    /// Mirror a stored entry with every identity on it, whoever is asking.
    #[must_use]
    pub fn omniscient(entry: &ActEntry) -> Self {
        let known = ActorIdentified::new(true);
        Self::mirror(entry, known, known)
    }

    // Copy an entry onto the wire, naming each identity only where it was earned.
    fn mirror(entry: &ActEntry, actor: ActorIdentified, interrupted: ActorIdentified) -> Self {
        Self {
            seq:        ActSeqNet::new(*entry.seq()),
            actor:      token_if(entry.actor(), actor),
            provenance: ActProvenanceNet::from_sim(entry.provenance(), interrupted),
            kind:       ActDeedKindNet::from_deed(entry.deed()),
        }
    }
}

/// The entity's token, or nothing when the reader could not identify it.
fn token_if(entity: bevy::prelude::Entity, identified: ActorIdentified) -> Option<GangerToken> {
    if *identified {
        Some(GangerToken::new(entity.to_bits()))
    } else {
        None
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
