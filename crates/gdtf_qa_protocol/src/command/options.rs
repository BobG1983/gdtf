use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::ShotName;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AwaitBudget(u64);

impl AwaitBudget {
        #[must_use]
    pub const fn new(seconds: u64) -> Self {
        Self(seconds)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureRider {
        pub name: Option<ShotName>,
}

impl CaptureRider {
        #[must_use]
    pub const fn new(name: Option<ShotName>) -> Self {
        Self { name }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunOptions {
        pub await_ready: Option<AwaitBudget>,
        pub capture:     Option<CaptureRider>,
}

impl RunOptions {
        #[must_use]
    pub const fn new(await_ready: Option<AwaitBudget>, capture: Option<CaptureRider>) -> Self {
        Self {
            await_ready,
            capture,
        }
    }

                        #[must_use]
    pub const fn is_plain(&self) -> bool {
        self.await_ready.is_none() && self.capture.is_none()
    }
}
