//! The act log's counting newtypes — the per-entry sequence number, the ring capacity,
//! and the overflow drop counter (GTW-727 C2 / C7).

use bevy::prelude::Deref;

/// The **sequence number** of one [`ActEntry`](super::ActEntry) — its position in the
/// battle's ordered act log.
///
/// Assigned SOLELY by [`ActLog::append`](super::ActLog::append), per battle, starting at
/// `0` and increasing by one per appended entry. A `u64` so both gap detection (a reader
/// whose cursor fell behind the ring's oldest entry) and battle restart detection are
/// plain subtraction, and so it can never wrap in a session.
///
/// A named newtype (`no-bare-types.md` rules 2/5): the inner is PRIVATE, read through the
/// derived [`Deref`] and built through [`ActSeq::new`] / [`ActSeq::START`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActSeq(u64);

impl ActSeq {
    /// The first sequence number of a battle — every [`ActLog`](super::ActLog) starts here.
    pub const START: Self = Self(0);

    /// Build a sequence number from its ordinal.
    #[must_use]
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }

    /// The sequence number one past this one — the log's next slot after appending here.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// How many entries separate `self` from the (older) `earlier` sequence number,
    /// saturating at zero when `earlier` is not actually earlier.
    ///
    /// The shared gap measure both the presenter's playback cursor and a QA output cursor
    /// use to size a skip (`my_position < log.oldest_seq()`).
    #[must_use]
    pub const fn distance_from(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl Default for ActSeq {
    /// A fresh sequence number is [`ActSeq::START`].
    fn default() -> Self {
        Self::START
    }
}

/// The **capacity** of the act log's ring — the maximum number of
/// [`ActEntry`](super::ActEntry)s retained before the oldest is dropped (GTW-727 C7).
///
/// A named newtype (`no-bare-types.md`): the inner is PRIVATE, read through the derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActLogCapacity(usize);

impl ActLogCapacity {
    /// The shipped ring capacity — `2048` entries.
    ///
    /// Sized against the worst normal-play burst rather than a whole battle: the enemy
    /// brain emits at most one act per tick and each act contributes a handful of entries
    /// (a declaration, its rounds, the consequence facts, the life transition), so 2048
    /// covers several seconds of uninterrupted acting at 60fps — far more than any
    /// presenter dwell backlog — while costing a bounded, small allocation. Retention is
    /// capacity-only: no reader's cursor pins the ring, so a QA client that never connects
    /// costs nothing and starves nobody.
    pub const DEFAULT: usize = 2048;

    /// Build a ring capacity from its entry count.
    #[must_use]
    pub const fn new(entries: usize) -> Self {
        Self(entries)
    }
}

impl Default for ActLogCapacity {
    /// The default ring capacity is [`ActLogCapacity::DEFAULT`] entries.
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// How many [`ActEntry`](super::ActEntry)s the ring has DROPPED to overflow since the
/// battle began (GTW-727 C7).
///
/// Incremented once per evicted oldest entry. A reader compares its own cursor against
/// [`ActLog::oldest_seq`](super::ActLog::oldest_seq) to detect that it personally fell off
/// the window; this counter is the log-wide total, for diagnostics.
///
/// A named newtype (`no-bare-types.md`): the inner is PRIVATE, read through the derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ActLogDropped(u32);

impl ActLogDropped {
    /// Build a drop count from its total.
    #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }

    /// Count one more dropped entry (saturating — the counter never wraps).
    pub const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}
