//! [`CommandName`] / [`CommandSummary`] — a command's identity on the wire (GTW-939).

use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A command's **name** — the word a client puts in a
/// [`RunCommand`](crate::envelope::RunCommand).
///
/// A private-inner newtype over `Cow<'static, str>` (no-bare-types), serde-transparent so
/// it rides the wire as a plain string. The `Cow` is what lets a host declare it as an
/// associated `const` from a `&'static str` while a client still deserializes an owned
/// one — ONE type instead of a static/owned pair that would have to be kept in step.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandName(Cow<'static, str>);

impl CommandName {
    /// Build a command name from a compile-time literal — the form a host's `const NAME`
    /// uses.
    #[must_use]
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// Build a command name from a client-supplied string.
    #[must_use]
    pub const fn from_owned(name: String) -> Self {
        Self(Cow::Owned(name))
    }

    /// This name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A command's one-line **summary** — what it does and when to reach for it.
///
/// Private-inner newtype over `Cow<'static, str>` (no-bare-types), serde-transparent.
/// Distinct from [`CommandName`]: a name selects a command, a summary explains it, and the
/// two must never be interchangeable at a call site.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandSummary(Cow<'static, str>);

impl CommandSummary {
    /// Build a summary from a compile-time literal — the form a host's `const SUMMARY`
    /// uses.
    #[must_use]
    pub const fn from_static(text: &'static str) -> Self {
        Self(Cow::Borrowed(text))
    }

    /// Build a summary from a runtime string — the form a decoding client produces.
    #[must_use]
    pub const fn from_owned(text: String) -> Self {
        Self(Cow::Owned(text))
    }

    /// This summary as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
