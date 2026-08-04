//! One logged act: sequence, actor, provenance, deed.

use bevy::prelude::Entity;

use super::{deed::ActDeed, provenance::ActProvenance, seq::ActSeq};

/// Fully sequenced act stored in the log.
#[derive(Debug, Clone, PartialEq)]
pub struct ActEntry {
    seq:        ActSeq,
    actor:      Entity,
    provenance: ActProvenance,
    deed:       ActDeed,
}

impl ActEntry {
    /// Build an entry (crate-internal).
    #[must_use]
    pub(super) const fn new(
        seq: ActSeq,
        actor: Entity,
        provenance: ActProvenance,
        deed: ActDeed,
    ) -> Self {
        Self {
            seq,
            actor,
            provenance,
            deed,
        }
    }

    /// Sequence number.
    #[must_use]
    pub const fn seq(&self) -> ActSeq {
        self.seq
    }

    /// Acting entity.
    #[must_use]
    pub const fn actor(&self) -> Entity {
        self.actor
    }

    /// Where the act came from.
    #[must_use]
    pub const fn provenance(&self) -> ActProvenance {
        self.provenance
    }

    /// What happened.
    #[must_use]
    pub const fn deed(&self) -> &ActDeed {
        &self.deed
    }
}

/// Act ready to append (no sequence yet).
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedAct {
    /// Acting entity.
    pub actor:      Entity,
    /// Origin of the act.
    pub provenance: ActProvenance,
    /// What happened.
    pub deed:       ActDeed,
}

impl RecordedAct {
    /// Build a recorded act.
    #[must_use]
    pub const fn new(actor: Entity, provenance: ActProvenance, deed: ActDeed) -> Self {
        Self {
            actor,
            provenance,
            deed,
        }
    }
}
