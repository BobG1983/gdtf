//! Menu and shell read payloads on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Whether game sound is on.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SoundNet(bool);

impl SoundNet {
    /// Build from a bool.
    #[must_use]
    pub const fn new(on: bool) -> Self {
        Self(on)
    }
}

/// Whether the screen has caught up with the act log.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CaughtUpNet(bool);

impl CaughtUpNet {
    /// Build from a bool.
    #[must_use]
    pub const fn new(caught_up: bool) -> Self {
        Self(caught_up)
    }
}
