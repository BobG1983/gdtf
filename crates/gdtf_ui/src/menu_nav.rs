use bevy::prelude::*;

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuName(String);

impl MenuName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuScreen {
        name: MenuName,
}

impl MenuScreen {
        #[must_use]
    pub const fn new(name: MenuName) -> Self {
        Self { name }
    }

        #[must_use]
    pub const fn name(&self) -> &MenuName {
        &self.name
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MenuItem;
