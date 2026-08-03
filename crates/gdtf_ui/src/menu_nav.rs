//! Named menu screens and selectable items.

use bevy::prelude::*;

/// Stable name for a menu screen.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuName(String);

impl MenuName {
    /// Wrap a name string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Marks a UI root as a named menu screen.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuScreen {
    name: MenuName,
}

impl MenuScreen {
    /// Build a screen with the given name.
    #[must_use]
    pub const fn new(name: MenuName) -> Self {
        Self { name }
    }

    /// Screen name.
    #[must_use]
    pub const fn name(&self) -> &MenuName {
        &self.name
    }
}

/// Marks a focusable menu entry.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MenuItem;
