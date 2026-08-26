//! Whether the open form's own draft resource is in the world this frame.

use bevy::prelude::Deref;

/// Whether the active mode's draft resource was in the world when the facts were sampled.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::net_qa) struct DraftInWorld(bool);

impl DraftInWorld {
    /// Wrap what the sampling frame found.
    pub(in crate::net_qa) const fn new(present: bool) -> Self {
        Self(present)
    }
}
