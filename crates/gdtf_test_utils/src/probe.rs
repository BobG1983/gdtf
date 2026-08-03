use std::marker::PhantomData;

use bevy::{
    app::{App, Last, Plugin},
    ecs::message::{Message, MessageReader},
    prelude::{ResMut, Resource},
};

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
        #[must_use]
    pub fn seen(&self) -> &[M] {
        &self.seen
    }

            pub fn clear(&mut self) {
        self.seen.clear();
    }
}

pub fn drain_message_probe<M: Message + Clone>(
    mut reader: MessageReader<M>,
    mut probe: ResMut<MessageProbe<M>>,
) {
    probe.seen.extend(reader.read().cloned());
}

#[must_use]
pub fn probed<M: Message + Clone>(app: &App) -> Vec<M> {
    app.world()
        .get_resource::<MessageProbe<M>>()
        .map(|probe| probe.seen.clone())
        .unwrap_or_default()
}

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
