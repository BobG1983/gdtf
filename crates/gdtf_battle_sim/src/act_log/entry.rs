use bevy::prelude::Entity;

use super::{deed::ActDeed, provenance::ActProvenance, seq::ActSeq};

#[derive(Debug, Clone, PartialEq)]
pub struct ActEntry {
        seq:        ActSeq,
        actor:      Entity,
        provenance: ActProvenance,
        deed:       ActDeed,
}

impl ActEntry {
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

        #[must_use]
    pub const fn seq(&self) -> ActSeq {
        self.seq
    }

            #[must_use]
    pub const fn actor(&self) -> Entity {
        self.actor
    }

        #[must_use]
    pub const fn provenance(&self) -> ActProvenance {
        self.provenance
    }

        #[must_use]
    pub const fn deed(&self) -> &ActDeed {
        &self.deed
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedAct {
        pub actor:      Entity,
        pub provenance: ActProvenance,
        pub deed:       ActDeed,
}

impl RecordedAct {
        #[must_use]
    pub const fn new(actor: Entity, provenance: ActProvenance, deed: ActDeed) -> Self {
        Self {
            actor,
            provenance,
            deed,
        }
    }
}
