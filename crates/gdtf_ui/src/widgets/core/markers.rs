//! Button state markers and labels.

use bevy::prelude::*;

/// Button is not interactive.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisabledButton;

/// Button is currently selected / pressed style.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActiveButton;

/// Caption text for a button spawn helper.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct ButtonLabel(String);

impl ButtonLabel {
    /// Wrap a label string.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// Consume into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}
