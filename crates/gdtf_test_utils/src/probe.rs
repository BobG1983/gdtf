//! Capture Bevy messages into a resource for test assertions.

use std::marker::PhantomData;

use bevy::{
    app::{App, Last, Plugin},
    ecs::message::{Message, MessageReader},
    prelude::{ResMut, Resource},
};

/// Accumulated messages of type `M` seen so far.
#[derive(Resource)]
pub struct MessageProbe<M: Message> {
    seen: Vec<M>,
}

impl<M: Message> Default for MessageProbe<M> {
    fn default() -> Self {
        Self { seen: Vec::new() }
    }
}

impl<M: Message> MessageProbe<M> {
    /// Messages captured so far.
    #[must_use]
    pub fn seen(&self) -> &[M] {
        &self.seen
    }

    /// Clear captured messages.
    pub fn clear(&mut self) {
        self.seen.clear();
    }
}

/// System that appends newly read messages into the probe.
pub fn drain_message_probe<M: Message + Clone>(
    mut reader: MessageReader<M>,
    mut probe: ResMut<MessageProbe<M>>,
) {
    probe.seen.extend(reader.read().cloned());
}

/// Snapshot of messages currently in the probe (empty if the resource is missing).
#[must_use]
pub fn probed<M: Message + Clone>(app: &App) -> Vec<M> {
    app.world()
        .get_resource::<MessageProbe<M>>()
        .map(|probe| probe.seen.clone())
        .unwrap_or_default()
}

/// Plugin that registers message `M` and drains it into [`MessageProbe`].
pub struct MessageProbePlugin<M: Message + Clone>(PhantomData<M>);

impl<M: Message + Clone> Default for MessageProbePlugin<M> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<M: Message + Clone> Plugin for MessageProbePlugin<M> {
    fn build(&self, app: &mut App) {
        app.add_message::<M>();
        app.init_resource::<MessageProbe<M>>();
        app.add_systems(Last, drain_message_probe::<M>);
    }
}
