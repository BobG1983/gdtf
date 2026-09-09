use core::ops::Deref;
use std::time::Instant;

/// The instant a wait runs out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::lifecycle) struct DeadlineAt(Instant);

impl DeadlineAt {
    /// Wrap the instant a wait runs out at.
    #[must_use]
    pub(in crate::lifecycle) const fn new(at: Instant) -> Self {
        Self(at)
    }
}

impl Deref for DeadlineAt {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
