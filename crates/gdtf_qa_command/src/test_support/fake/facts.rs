//! [`FakeFacts`] — the fake host's per-frame facts, and the two sample values the suite
//! reads availability across.

use bevy::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Whether the fake host's model is loaded this frame.
///
/// Private-inner newtype over `bool` (no-bare-types).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FakeReady(bool);

impl FakeReady {
    /// Build a readiness fact.
    #[must_use]
    pub const fn new(loaded: bool) -> Self {
        Self(loaded)
    }
}

/// The fake host's current level index.
///
/// Private-inner newtype over `u8` (no-bare-types). A second, independent fact so the two
/// sample values below differ in more than one way and a predicate can key on either.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FakeLevel(u8);

impl FakeLevel {
    /// Build a level index.
    #[must_use]
    pub const fn new(level: u8) -> Self {
        Self(level)
    }
}

/// The per-frame facts EVERY fake command's availability predicate reads.
///
/// A `Resource` as well as a facts type, because the fake handlers read it the way a real
/// host's handlers read the world: through an ordinary `Res`, never `&mut World`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FakeFacts {
    /// Whether the fake model is loaded.
    ready: FakeReady,
    /// The current level index.
    level: FakeLevel,
}

impl FakeFacts {
    /// Build a facts sample.
    #[must_use]
    pub const fn new(ready: FakeReady, level: FakeLevel) -> Self {
        Self { ready, level }
    }

    /// Whether the fake model is loaded.
    #[must_use]
    pub const fn ready(&self) -> FakeReady {
        self.ready
    }

    /// The current level index.
    #[must_use]
    pub const fn level(&self) -> FakeLevel {
        self.level
    }
}

/// The facts of a fake host with nothing loaded — the state that makes
/// [`FakeCell`](super::FakeCell) and [`FakeEcho`](super::FakeEcho) unavailable.
#[must_use]
pub const fn fake_facts_unloaded() -> FakeFacts {
    FakeFacts::new(FakeReady::new(false), FakeLevel::new(0))
}

/// The facts of a fake host with its model loaded — the state that makes every fake
/// command available.
#[must_use]
pub const fn fake_facts_loaded() -> FakeFacts {
    FakeFacts::new(FakeReady::new(true), FakeLevel::new(2))
}
