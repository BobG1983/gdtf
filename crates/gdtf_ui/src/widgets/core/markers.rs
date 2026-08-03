use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisabledButton;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActiveButton;

#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct ButtonLabel(String);

impl ButtonLabel {
        #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

            #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}
