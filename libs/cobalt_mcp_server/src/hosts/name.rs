//! The name a registered host answers to in tool calls.

use core::ops::Deref;

/// Name a host is registered under, as a tool call spells it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostName(String);

impl HostName {
    /// Wrap a registered name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    /// The name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for HostName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::fmt::Display for HostName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}
