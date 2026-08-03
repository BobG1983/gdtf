//! Host facts for the fake command set.

use bevy::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Whether the fake host is loaded.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FakeReady(bool);

impl FakeReady {
    /// Wrap a loaded flag.
    #[must_use]
    pub const fn new(loaded: bool) -> Self {
        Self(loaded)
    }
}

/// Fake world level.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FakeLevel(u8);

impl FakeLevel {
    /// Wrap a level value.
    #[must_use]
    pub const fn new(level: u8) -> Self {
        Self(level)
    }
}

/// Bevy resource holding fake host readiness and level.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FakeFacts {
    ready: FakeReady,
    level: FakeLevel,
}

impl FakeFacts {
    /// Build facts from ready and level.
    #[must_use]
    pub const fn new(ready: FakeReady, level: FakeLevel) -> Self {
        Self { ready, level }
    }

    /// Ready flag.
    #[must_use]
    pub const fn ready(&self) -> FakeReady {
        self.ready
    }

    /// Level value.
    #[must_use]
    pub const fn level(&self) -> FakeLevel {
        self.level
    }
}

/// Facts with ready=false and level 0.
#[must_use]
pub const fn fake_facts_unloaded() -> FakeFacts {
    FakeFacts::new(FakeReady::new(false), FakeLevel::new(0))
}

/// Facts with ready=true and level 2.
#[must_use]
pub const fn fake_facts_loaded() -> FakeFacts {
    FakeFacts::new(FakeReady::new(true), FakeLevel::new(2))
}
