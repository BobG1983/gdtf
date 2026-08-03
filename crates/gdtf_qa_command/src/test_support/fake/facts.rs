use bevy::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FakeReady(bool);

impl FakeReady {
        #[must_use]
    pub const fn new(loaded: bool) -> Self {
        Self(loaded)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FakeLevel(u8);

impl FakeLevel {
        #[must_use]
    pub const fn new(level: u8) -> Self {
        Self(level)
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FakeFacts {
        ready: FakeReady,
        level: FakeLevel,
}

impl FakeFacts {
        #[must_use]
    pub const fn new(ready: FakeReady, level: FakeLevel) -> Self {
        Self { ready, level }
    }

        #[must_use]
    pub const fn ready(&self) -> FakeReady {
        self.ready
    }

        #[must_use]
    pub const fn level(&self) -> FakeLevel {
        self.level
    }
}

#[must_use]
pub const fn fake_facts_unloaded() -> FakeFacts {
    FakeFacts::new(FakeReady::new(false), FakeLevel::new(0))
}

#[must_use]
pub const fn fake_facts_loaded() -> FakeFacts {
    FakeFacts::new(FakeReady::new(true), FakeLevel::new(2))
}
