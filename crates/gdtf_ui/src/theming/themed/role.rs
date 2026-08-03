//! Theme roles attached to UI nodes.

use bevy::prelude::*;

/// Which theme bucket a node draws from.
///
/// The `#[default]` ([`ThemeRole::Background`]) is a spawn-seed sentinel only.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeRole {
    /// Screen background (also the default spawn-seed sentinel).
    #[default]
    Background,
    /// Panel chrome.
    Panel,
    /// Button chrome.
    Button,
    /// Button caption text.
    ButtonText,
    /// Title text.
    Title,
    /// Body text.
    Text,
}

/// Marks an entity as themed with a role.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Themed(ThemeRole);

impl Themed {
    /// Attach a theme role.
    #[must_use]
    pub const fn new(role: ThemeRole) -> Self {
        Self(role)
    }
}

/// System sets for theme application ordering.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSystems {
    /// Apply theme colors to themed nodes.
    ApplyTheme,
}
