//! Sequence numbers and ring-buffer capacity for the act log.

use bevy::prelude::Deref;

/// Monotonic sequence id for a logged act.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActSeq(u64);

impl ActSeq {
    /// First sequence value.
    pub const START: Self = Self(0);

    /// Wrap a sequence number.
    #[must_use]
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }

    /// Next sequence (saturating).
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Distance from an earlier sequence.
    #[must_use]
    pub const fn distance_from(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl Default for ActSeq {
    fn default() -> Self {
        Self::START
    }
}

/// Max entries retained in the ring buffer.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActLogCapacity(usize);

impl ActLogCapacity {
    /// Default capacity.
    pub const DEFAULT: usize = 2048;

    /// Wrap a capacity.
    #[must_use]
    pub const fn new(entries: usize) -> Self {
        Self(entries)
    }
}

impl Default for ActLogCapacity {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// Count of entries dropped when the log exceeds capacity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ActLogDropped(u32);

impl ActLogDropped {
    /// Wrap a drop count.
    #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }

    /// Increment by one (saturating).
    pub const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}
