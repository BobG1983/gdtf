//! Which factions could observe a logged act, and whose names they earned by it.
//! Resolved when the act is appended and never re-resolved.

use bevy::platform::collections::HashSet;

use crate::{
    ganger::Faction,
    visibility::{ActObserved, ActorIdentified},
};

/// The factions that could see something. A faction absent from the set did not see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchingFactions(HashSet<Faction>);

impl WatchingFactions {
    /// Build from the factions that could see it.
    #[must_use]
    pub fn new(factions: impl IntoIterator<Item = Faction>) -> Self {
        Self(factions.into_iter().collect())
    }

    /// Nobody could see it.
    #[must_use]
    pub fn nobody() -> Self {
        Self(HashSet::default())
    }

    /// Whether this faction is in the set. Absent reads as no, so the record fails closed.
    #[must_use]
    pub fn contains_faction(&self, faction: &Faction) -> bool {
        self.0.contains(faction)
    }
}

/// Who could observe an act, and whose identity each of them could put to it.
/// Observing and identifying are separate sets, so one can hold without the other.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActWitnesses {
    observed:    WatchingFactions,
    actor:       WatchingFactions,
    interrupted: WatchingFactions,
}

impl ActWitnesses {
    /// Build from the factions that observed the act and those that could name its actor.
    #[must_use]
    pub fn new(observed: WatchingFactions, actor: WatchingFactions) -> Self {
        Self {
            observed,
            actor,
            interrupted: WatchingFactions::nobody(),
        }
    }

    /// Add the factions that could name the actor a reaction interrupted.
    #[must_use]
    pub fn interrupting(self, interrupted: WatchingFactions) -> Self {
        Self {
            interrupted,
            ..self
        }
    }

    /// Nobody observed it — what an entry records when no cell of it resolved.
    #[must_use]
    pub fn unseen() -> Self {
        Self::default()
    }

    /// Whether this faction could observe the act.
    #[must_use]
    pub fn observed_by(&self, faction: Faction) -> ActObserved {
        ActObserved::new(self.observed.contains_faction(&faction))
    }

    /// Whether this faction could put a name to the acting entity.
    #[must_use]
    pub fn identifies_actor(&self, faction: Faction) -> ActorIdentified {
        ActorIdentified::new(self.actor.contains_faction(&faction))
    }

    /// Whether this faction could put a name to the actor a reaction interrupted.
    #[must_use]
    pub fn identifies_interrupted(&self, faction: Faction) -> ActorIdentified {
        ActorIdentified::new(self.interrupted.contains_faction(&faction))
    }
}
