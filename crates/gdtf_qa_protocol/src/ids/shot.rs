//! Screenshot / capture shot name on the wire.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Logical name for a QA shot.
#[derive(
    schemars::JsonSchema, Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ShotName(String);

impl ShotName {
    /// Wrap an owned name string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
