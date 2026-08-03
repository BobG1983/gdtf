//! Pending act intent queue.

use bevy::prelude::*;

use super::ActIntent;

/// Queue of act intents gathered this frame.
#[derive(Resource, Debug, Default)]
pub struct PendingActIntent(Vec<ActIntent>);

impl PendingActIntent {
    /// Push an intent onto the queue.
    pub fn push(&mut self, intent: ActIntent) {
        self.0.push(intent);
    }

    pub(crate) fn drain(&mut self) -> Vec<ActIntent> {
        core::mem::take(&mut self.0)
    }

    /// Whether the queue is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
