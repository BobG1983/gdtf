use bevy::prelude::*;

/// The `#[default]` ([`ThemeRole::Background`]) is a **spawn-seed sentinel only**,
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeRole {
                    /// Also the `#[default]` spawn-seed sentinel (see the type doc) — never a
        #[default]
    Background,
                Panel,
                    Button,
                ButtonText,
                Title,
                Text,
}

#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Themed(ThemeRole);

impl Themed {
        #[must_use]
    pub const fn new(role: ThemeRole) -> Self {
        Self(role)
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSystems {
            ApplyTheme,
}
