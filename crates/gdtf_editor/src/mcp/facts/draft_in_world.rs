//! Whether the open form's own draft resource is in the world this frame.

use bevy::prelude::Deref;

/// Whether the active mode's draft resource was in the world when the facts were sampled.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mcp) struct DraftInWorld(bool);

impl DraftInWorld {
    /// Wrap what the sampling frame found.
    pub(in crate::mcp) const fn new(present: bool) -> Self {
        Self(present)
    }
}
