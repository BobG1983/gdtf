//! String newtypes used by injury content.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Stable injury name key.
#[derive(Deref, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InjuryName(String);

impl InjuryName {
    /// Build from a string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Short text shown in a UI popup when the injury is inflicted.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PopupText(String);

impl PopupText {
    /// Build from a string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}

/// Text written to the act log.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LogText(String);

impl LogText {
    /// Build from a string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}

/// Longer description shown when inspecting the injury.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InspectText(String);

impl InspectText {
    /// Build from a string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}
