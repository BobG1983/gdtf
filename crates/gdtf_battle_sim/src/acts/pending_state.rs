//! What a dispatcher has already driven its targets to inside one run.

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Entity},
};

/// The states one dispatcher run has already written, keyed by the entity each one targets.
#[derive(Deref, Debug, Clone, Default)]
pub(super) struct PendingStates<S>(HashMap<Entity, S>);

impl<S: Copy> PendingStates<S> {
    /// A run that has written nothing yet.
    pub(super) const fn new() -> Self {
        Self(HashMap::new())
    }

    /// The state this run wrote for `target`, falling back to `queried` when it wrote none.
    pub(super) fn state_of(&self, target: Entity, queried: S) -> S {
        self.0.get(&target).copied().unwrap_or(queried)
    }

    /// Record the state a request has just driven `target` to.
    pub(super) fn record(&mut self, target: Entity, state: S) {
        self.0.insert(target, state);
    }
}
