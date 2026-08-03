use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::*};

use super::{
    entry::{ActEntry, RecordedAct},
    facts::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts},
    seq::{ActLogCapacity, ActLogDropped, ActSeq},
};
use crate::ganger::LifeState;

#[derive(Resource, Debug)]
pub struct ActLog {
        entries:        VecDeque<ActEntry>,
        next_seq:       ActSeq,
        capacity:       ActLogCapacity,
        dropped:        ActLogDropped,
        prior_pose:     HashMap<Entity, PoseFacts>,
        prior_vitals:   HashMap<Entity, VitalsFacts>,
        prior_magazine: HashMap<Entity, MagazineFacts>,
        prior_life:     HashMap<Entity, LifeState>,
        prior_position: HashMap<Entity, PositionFacts>,
}

impl ActLog {
        #[must_use]
    pub fn new(capacity: ActLogCapacity) -> Self {
        Self {
            entries: VecDeque::new(),
            next_seq: ActSeq::START,
            capacity,
            dropped: ActLogDropped::default(),
            prior_pose: HashMap::default(),
            prior_vitals: HashMap::default(),
            prior_magazine: HashMap::default(),
            prior_life: HashMap::default(),
            prior_position: HashMap::default(),
        }
    }

                        pub fn append(&mut self, act: RecordedAct) -> ActSeq {
        let seq = self.next_seq;
        self.entries
            .push_back(ActEntry::new(seq, act.actor, act.provenance, act.deed));
        self.next_seq = seq.next();
        while self.entries.len() > *self.capacity {
            if self.entries.pop_front().is_some() {
                self.dropped.increment();
            }
        }
        seq
    }

                            pub fn since(&self, cursor: ActSeq) -> impl Iterator<Item = &ActEntry> {
        self.entries.iter().skip_while(move |e| e.seq() < cursor)
    }

        #[must_use]
    pub fn at(&self, seq: ActSeq) -> Option<&ActEntry> {
        self.entries.iter().find(|entry| entry.seq() == seq)
    }

            #[must_use]
    pub const fn head(&self) -> ActSeq {
        self.next_seq
    }

            #[must_use]
    pub fn oldest_seq(&self) -> ActSeq {
        self.entries.front().map_or(self.next_seq, ActEntry::seq)
    }

        #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

        #[must_use]
    pub const fn dropped(&self) -> ActLogDropped {
        self.dropped
    }

        #[must_use]
    pub const fn capacity(&self) -> ActLogCapacity {
        self.capacity
    }

                        pub fn note_pose(&mut self, entity: Entity, pose: PoseFacts) -> bool {
        match self.prior_pose.insert(entity, pose) {
            Some(prior) => prior != pose,
            None => false,
        }
    }

                pub fn note_vitals(&mut self, entity: Entity, vitals: &VitalsFacts) -> bool {
        match self.prior_vitals.insert(entity, vitals.clone()) {
            Some(prior) => &prior != vitals,
            None => false,
        }
    }

                    pub fn note_magazine(&mut self, entity: Entity, magazine: MagazineFacts) -> bool {
        match self.prior_magazine.insert(entity, magazine) {
            Some(prior) => prior != magazine,
            None => false,
        }
    }

                    pub fn note_position(&mut self, entity: Entity, position: PositionFacts) -> bool {
        match self.prior_position.insert(entity, position) {
            Some(prior) => prior != position,
            None => false,
        }
    }

                                pub fn note_life(&mut self, entity: Entity, life: LifeState) -> Option<LifeState> {
        self.prior_life
            .insert(entity, life)
            .filter(|prior| *prior != life)
    }
}

impl Default for ActLog {
        fn default() -> Self {
        Self::new(ActLogCapacity::default())
    }
}
