//! [`ActLog`] — the battle-lifetime ordered record of everything that happened, plus the
//! prior-value maps the query-sourced recorders detect transitions against (GTW-727
//! C6–C8 / C11).

use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::*};

use super::{
    entry::{ActEntry, RecordedAct},
    facts::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts},
    seq::{ActLogCapacity, ActLogDropped, ActSeq},
};
use crate::ganger::LifeState;

/// The battle's ordered ACT LOG — every recorded deed, in the order the sim produced it.
///
/// A battle-lifetime [`Resource`]: inserted on the successful-setup `Ok` path beside
/// [`BattleInProgress`](crate::battle::BattleInProgress) and removed alongside it on
/// teardown, so per-battle sequence numbering and the prior-value maps reset for free.
/// Every reader takes `Option<Res<ActLog>>` or gates on `resource_exists`
/// (`bevy-traps.md` #1).
///
/// ## Retention is CAPACITY-ONLY
///
/// Overflow drops the OLDEST entry and counts it ([`dropped`](Self::dropped)). No
/// consumer's cursor pins the ring: the log never stalls a writer, never coalesces
/// entries, and never applies back-pressure — the sim appends and returns. A reader that
/// falls behind detects it by comparing its own cursor against
/// [`oldest_seq`](Self::oldest_seq) and degrades (jumping forward) rather than blocking.
/// This is what keeps two independent cursors — a presenter playback cursor and a QA
/// output cursor — coexisting over ONE buffer with neither able to starve the other.
///
/// ## The prior-value maps
///
/// Three of the recorded after-values (posture, vitals, magazine) and the life-state
/// transition have no owning message, so they are detected as CHANGES against the values
/// last recorded here. A FIRST observation seeds the map and records NOTHING — without
/// that, spawning a roster would flood the log, since situation setup writes facing,
/// stance, aiming, position and life state on every ganger at spawn.
#[derive(Resource, Debug)]
pub struct ActLog {
    /// The retained entries, oldest first.
    entries:        VecDeque<ActEntry>,
    /// The sequence number the NEXT appended entry will carry.
    next_seq:       ActSeq,
    /// How many entries the ring retains before evicting the oldest.
    capacity:       ActLogCapacity,
    /// How many entries have been evicted to overflow this battle.
    dropped:        ActLogDropped,
    /// The last recorded posture per ganger.
    prior_pose:     HashMap<Entity, PoseFacts>,
    /// The last recorded vitals per ganger.
    prior_vitals:   HashMap<Entity, VitalsFacts>,
    /// The last recorded magazine per weapon entity.
    prior_magazine: HashMap<Entity, MagazineFacts>,
    /// The last recorded life state per ganger.
    prior_life:     HashMap<Entity, LifeState>,
    /// The last recorded position per ganger.
    prior_position: HashMap<Entity, PositionFacts>,
}

impl ActLog {
    /// Build an empty log with `capacity` retained entries.
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

    /// APPEND one recorded act, stamping it with the next sequence number.
    ///
    /// The ONLY way an entry enters the log, and the only place an [`ActSeq`] is minted.
    /// Never blocks and never coalesces: an append past [`capacity`](Self::capacity)
    /// evicts the oldest entry and counts it.
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

    /// Every retained entry at or after `cursor`, oldest first — a NON-DESTRUCTIVE read.
    ///
    /// Reading does not consume, so two independent cursors (the presenter's playback
    /// cursor and a QA output cursor) coexist over one buffer. A cursor that has fallen
    /// off the window (`cursor < oldest_seq()`) simply yields everything retained; the
    /// caller detects the gap by comparing against [`oldest_seq`](Self::oldest_seq).
    pub fn since(&self, cursor: ActSeq) -> impl Iterator<Item = &ActEntry> {
        self.entries.iter().skip_while(move |e| e.seq() < cursor)
    }

    /// The entry at exactly `seq`, if it is still retained.
    #[must_use]
    pub fn at(&self, seq: ActSeq) -> Option<&ActEntry> {
        self.entries.iter().find(|entry| entry.seq() == seq)
    }

    /// The sequence number ONE PAST the last appended entry — what a fully caught-up
    /// cursor equals.
    #[must_use]
    pub const fn head(&self) -> ActSeq {
        self.next_seq
    }

    /// The sequence number of the oldest RETAINED entry (equal to [`head`](Self::head)
    /// when the log is empty).
    #[must_use]
    pub fn oldest_seq(&self) -> ActSeq {
        self.entries.front().map_or(self.next_seq, ActEntry::seq)
    }

    /// How many entries the ring currently retains.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ring currently retains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries have been evicted to overflow this battle.
    #[must_use]
    pub const fn dropped(&self) -> ActLogDropped {
        self.dropped
    }

    /// How many entries the ring retains before evicting the oldest.
    #[must_use]
    pub const fn capacity(&self) -> ActLogCapacity {
        self.capacity
    }

    /// Record `pose` as `entity`'s posture and report whether that was a TRANSITION.
    ///
    /// A first observation SEEDS the map and returns `false` (no entry is recorded), which
    /// is what keeps a roster spawn — which writes facing, stance and aiming on every
    /// ganger at once — from flooding the log.
    pub fn note_pose(&mut self, entity: Entity, pose: PoseFacts) -> bool {
        match self.prior_pose.insert(entity, pose) {
            Some(prior) => prior != pose,
            None => false,
        }
    }

    /// Record `vitals` as `entity`'s vitals and report whether that was a TRANSITION.
    ///
    /// Seeds silently on a first observation, exactly like [`note_pose`](Self::note_pose).
    pub fn note_vitals(&mut self, entity: Entity, vitals: &VitalsFacts) -> bool {
        match self.prior_vitals.insert(entity, vitals.clone()) {
            Some(prior) => &prior != vitals,
            None => false,
        }
    }

    /// Record `magazine` as weapon `entity`'s magazine and report whether that was a
    /// TRANSITION.
    ///
    /// Seeds silently on a first observation, exactly like [`note_pose`](Self::note_pose).
    pub fn note_magazine(&mut self, entity: Entity, magazine: MagazineFacts) -> bool {
        match self.prior_magazine.insert(entity, magazine) {
            Some(prior) => prior != magazine,
            None => false,
        }
    }

    /// Record `position` as `entity`'s position and report whether that was a TRANSITION.
    ///
    /// Seeds silently on a first observation, exactly like [`note_pose`](Self::note_pose),
    /// so a spawned roster records no movement.
    pub fn note_position(&mut self, entity: Entity, position: PositionFacts) -> bool {
        match self.prior_position.insert(entity, position) {
            Some(prior) => prior != position,
            None => false,
        }
    }

    /// Record `life` as `entity`'s life state and return the state it LEFT, if this was a
    /// transition.
    ///
    /// Seeds silently on a first observation (returning [`None`]), so a spawned roster
    /// records no life changes. The returned prior is the `from` half of
    /// [`ActDeed::LifeChanged`](super::ActDeed::LifeChanged) — a genuine transition, which
    /// a post-act snapshot could never produce.
    pub fn note_life(&mut self, entity: Entity, life: LifeState) -> Option<LifeState> {
        self.prior_life
            .insert(entity, life)
            .filter(|prior| *prior != life)
    }
}

impl Default for ActLog {
    /// A fresh log with the shipped [`ActLogCapacity::DEFAULT`] ring.
    fn default() -> Self {
        Self::new(ActLogCapacity::default())
    }
}
