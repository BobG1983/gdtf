//! Act log resource and prior-state tracking.

use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::*};

use super::{
    entry::{ActEntry, RecordedAct},
    facts::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts},
    seq::ActSeq,
};
use crate::ganger::LifeState;

/// Every act a battle has recorded, in order, plus the prior-state maps.
#[derive(Resource, Debug, Default)]
pub struct ActLog {
    entries:        VecDeque<ActEntry>,
    next_seq:       ActSeq,
    prior_pose:     HashMap<Entity, PoseFacts>,
    prior_vitals:   HashMap<Entity, VitalsFacts>,
    prior_magazine: HashMap<Entity, MagazineFacts>,
    prior_life:     HashMap<Entity, LifeState>,
    prior_position: HashMap<Entity, PositionFacts>,
    prior_seen:     HashMap<Entity, SquadSees>,
}

/// Whether the player squad could see a ganger the last time the log looked.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquadSees(bool);

impl SquadSees {
    /// Wrap a seen flag.
    #[must_use]
    pub const fn new(seen: bool) -> Self {
        Self(seen)
    }
}

impl ActLog {
    /// Append an act and return its sequence. Every entry is kept for the battle.
    pub fn append(&mut self, act: RecordedAct) -> ActSeq {
        let seq = self.next_seq;
        self.entries.push_back(ActEntry::new(
            seq,
            act.actor,
            act.provenance,
            act.deed,
            act.witnesses,
        ));
        self.next_seq = seq.next();
        seq
    }

    /// Entries at or after `cursor`.
    pub fn since(&self, cursor: ActSeq) -> impl Iterator<Item = &ActEntry> {
        self.entries.iter().skip_while(move |e| e.seq() < cursor)
    }

    /// Entry with this exact sequence, if the log has reached it.
    #[must_use]
    pub fn at(&self, seq: ActSeq) -> Option<&ActEntry> {
        self.entries.iter().find(|entry| entry.seq() == seq)
    }

    /// Next sequence that would be assigned.
    #[must_use]
    pub const fn head(&self) -> ActSeq {
        self.next_seq
    }

    /// Oldest sequence the log holds (or head if empty).
    #[must_use]
    pub fn oldest_seq(&self) -> ActSeq {
        self.entries.front().map_or(self.next_seq, ActEntry::seq)
    }

    /// Number of entries held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Note pose; returns true if it changed from the prior value.
    pub fn note_pose(&mut self, entity: Entity, pose: PoseFacts) -> bool {
        match self.prior_pose.insert(entity, pose) {
            Some(prior) => prior != pose,
            None => false,
        }
    }

    /// Note vitals; returns true if they changed.
    pub fn note_vitals(&mut self, entity: Entity, vitals: &VitalsFacts) -> bool {
        match self.prior_vitals.insert(entity, vitals.clone()) {
            Some(prior) => &prior != vitals,
            None => false,
        }
    }

    /// Note magazine; returns true if it changed.
    pub fn note_magazine(&mut self, entity: Entity, magazine: MagazineFacts) -> bool {
        match self.prior_magazine.insert(entity, magazine) {
            Some(prior) => prior != magazine,
            None => false,
        }
    }

    /// Note position; returns true if it changed.
    pub fn note_position(&mut self, entity: Entity, position: PositionFacts) -> bool {
        match self.prior_position.insert(entity, position) {
            Some(prior) => prior != position,
            None => false,
        }
    }

    /// Note squad sight of a ganger; true when a hidden one has just come into view.
    /// The first look seeds the map and reports nothing.
    pub fn note_seen(&mut self, entity: Entity, seen: SquadSees) -> bool {
        match self.prior_seen.insert(entity, seen) {
            Some(prior) => !*prior && *seen,
            None => false,
        }
    }

    /// Note life state; returns the previous value if it changed.
    pub fn note_life(&mut self, entity: Entity, life: LifeState) -> Option<LifeState> {
        self.prior_life
            .insert(entity, life)
            .filter(|prior| *prior != life)
    }
}
