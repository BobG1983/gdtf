//! One logged act: sequence, actor, provenance, deed.

use bevy::prelude::Entity;

use super::{deed::ActDeed, provenance::ActProvenance, seq::ActSeq, witness::ActWitnesses};

/// Fully sequenced act stored in the log.
#[derive(Debug, Clone, PartialEq)]
pub struct ActEntry {
    seq:        ActSeq,
    actor:      Entity,
    provenance: ActProvenance,
    deed:       ActDeed,
    witnesses:  ActWitnesses,
}

impl ActEntry {
    /// Build an entry (crate-internal).
    #[must_use]
    pub(super) const fn new(
        seq: ActSeq,
        actor: Entity,
        provenance: ActProvenance,
        deed: ActDeed,
        witnesses: ActWitnesses,
    ) -> Self {
        Self {
            seq,
            actor,
            provenance,
            deed,
            witnesses,
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

    /// Who could observe the act when it was appended, and whom they could name.
    #[must_use]
    pub const fn witnesses(&self) -> &ActWitnesses {
        &self.witnesses
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
    /// Who could observe it, resolved from the cells it touched.
    pub witnesses:  ActWitnesses,
}

impl RecordedAct {
    /// Build a recorded act from who could observe it.
    #[must_use]
    pub const fn new(
        actor: Entity,
        provenance: ActProvenance,
        deed: ActDeed,
        witnesses: ActWitnesses,
    ) -> Self {
        Self {
            actor,
            provenance,
            deed,
            witnesses,
        }
    }
}
