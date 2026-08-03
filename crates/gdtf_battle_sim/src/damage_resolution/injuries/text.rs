use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

// INJURY authoring mode writes an edited `InjuryDef` / `InjuryWeighting` back to

/// value): private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
#[derive(Deref, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InjuryName(String);

impl InjuryName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// [`Deref`]; `#[serde(transparent)]` parses a bare RON string. The presenter
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PopupText(String);

impl PopupText {
        #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}

/// `#[serde(transparent)]` parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LogText(String);

impl LogText {
        #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}

/// `#[serde(transparent)]` parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InspectText(String);

impl InspectText {
        #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }
}
