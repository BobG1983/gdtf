//! Filename stem a caller asks a capture to use.

use core::ops::Deref;

/// Filename stem requested for one capture.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShotStem(String);

impl ShotStem {
    /// Wrap a requested stem.
    #[must_use]
    pub fn new(stem: impl Into<String>) -> Self {
        Self(stem.into())
    }
}

impl Deref for ShotStem {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}
