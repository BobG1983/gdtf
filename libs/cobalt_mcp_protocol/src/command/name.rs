//! Command identity and short summary text.

use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Stable command id on the wire.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandName(Cow<'static, str>);

impl CommandName {
    /// Borrow a static name.
    #[must_use]
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// Own a dynamic name.
    #[must_use]
    pub const fn from_owned(name: String) -> Self {
        Self(Cow::Owned(name))
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One-line human summary of a command.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandSummary(Cow<'static, str>);

impl CommandSummary {
    /// Borrow a static summary.
    #[must_use]
    pub const fn from_static(text: &'static str) -> Self {
        Self(Cow::Borrowed(text))
    }

    /// Own a dynamic summary.
    #[must_use]
    pub const fn from_owned(text: String) -> Self {
        Self(Cow::Owned(text))
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
