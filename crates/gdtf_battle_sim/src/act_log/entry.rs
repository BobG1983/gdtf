//! [`ActEntry`] — one ordered record in the act log: WHO did WHAT, WHY, and WHEN
//! (GTW-727 C3).

use bevy::prelude::Entity;

use super::{deed::ActDeed, provenance::ActProvenance, seq::ActSeq};

/// ONE recorded act — the log's element.
///
/// Fields are PRIVATE and read through accessors: the sequence number is assigned solely
/// by [`ActLog::append`](super::ActLog::append), so an entry constructed with an arbitrary
/// `seq` would break the ordering guarantee every consumer's cursor rests on.
///
/// Derives [`Debug`] + [`Clone`] + [`PartialEq`] and deliberately NOT [`Eq`] / [`Hash`]:
/// [`ActDeed::RoundResolved`] boxes a [`ShotFired`](crate::shot_fired::ShotFired) whose
/// [`SimPos`](crate::metric::SimPos) / [`ShotDir`](crate::sample_cone::ShotDir) hold
/// `f32`, so equality is bit-wise — the reasoning already recorded on `ShotFired` itself
/// (`shot_pipeline/shot_fired.rs`: "Derives `PartialEq` (NOT `Eq`: the `SimPos` /
/// `ShotDir` hold `f32`, so equality is bit-wise, the seeded-replay property)"). Entries
/// are read in order and compared with `==`, never keyed in a hashed or ordered set.
#[derive(Debug, Clone, PartialEq)]
pub struct ActEntry {
    /// This entry's position in the battle's log.
    seq:        ActSeq,
    /// The entity the deed is ABOUT.
    actor:      Entity,
    /// Why the act happened.
    provenance: ActProvenance,
    /// What happened.
    deed:       ActDeed,
}

impl ActEntry {
    /// Build an entry for `actor`'s `deed`, caused by `provenance`, at sequence `seq`.
    ///
    /// `pub(super)` — only [`ActLog::append`](super::ActLog::append) stamps a sequence
    /// number, so no caller outside this module can mint an out-of-order entry.
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

    /// This entry's sequence number.
    #[must_use]
    pub const fn seq(&self) -> ActSeq {
        self.seq
    }

    /// The entity this entry is about — the acting ganger, or (for
    /// [`ActDeed::MagazineChanged`]) the weapon entity whose magazine settled.
    #[must_use]
    pub const fn actor(&self) -> Entity {
        self.actor
    }

    /// Why the act happened.
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

/// One deed with its cause, before the log stamps a sequence number onto it — what a
/// per-family recorder hands to [`ActLog::append`](super::ActLog::append).
///
/// Keeping the un-sequenced triple its own type is what makes "[`ActSeq`] is assigned
/// SOLELY by `append`" (C2) a property of the types rather than a convention: a recorder
/// literally cannot name a sequence number.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedAct {
    /// The entity the deed is about.
    pub actor:      Entity,
    /// Why the act happened.
    pub provenance: ActProvenance,
    /// What happened.
    pub deed:       ActDeed,
}

impl RecordedAct {
    /// Build an un-sequenced record for `actor`'s `deed`, caused by `provenance`.
    #[must_use]
    pub const fn new(actor: Entity, provenance: ActProvenance, deed: ActDeed) -> Self {
        Self {
            actor,
            provenance,
            deed,
        }
    }
}
