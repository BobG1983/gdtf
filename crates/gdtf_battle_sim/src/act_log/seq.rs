//! Sequence numbers for the act log.

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
