//! Optional flags on a run request (await budget, capture rider).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::ShotName;

/// Seconds the client is willing to wait for readiness.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AwaitBudget(u64);

impl AwaitBudget {
    /// Wrap a second count.
    #[must_use]
    pub const fn new(seconds: u64) -> Self {
        Self(seconds)
    }
}

/// Optional screenshot capture after the command.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureRider {
    /// Optional shot name override.
    pub name: Option<ShotName>,
}

impl CaptureRider {
    /// Build a capture rider.
    #[must_use]
    pub const fn new(name: Option<ShotName>) -> Self {
        Self { name }
    }
}

/// Extra options on [`crate::message::RunCommand`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunOptions {
    /// Wait for readiness up to this many seconds.
    pub await_ready: Option<AwaitBudget>,
    /// Take a screenshot after the command.
    pub capture:     Option<CaptureRider>,
}

impl RunOptions {
    /// Build options from parts.
    #[must_use]
    pub const fn new(await_ready: Option<AwaitBudget>, capture: Option<CaptureRider>) -> Self {
        Self {
            await_ready,
            capture,
        }
    }

    /// True when no optional fields are set.
    #[must_use]
    pub const fn is_plain(&self) -> bool {
        self.await_ready.is_none() && self.capture.is_none()
    }
}
