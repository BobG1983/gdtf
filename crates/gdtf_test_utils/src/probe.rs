//! The generic [`MessageProbe<M>`] — the ONE message-collection idiom replacing the
//! hand-rolled per-message `*Probe(Vec<M>)` resources + drain closures (GTW-576).
//!
//! [`MessageProbePlugin::<M>`] registers the probe resource and a drain system in
//! [`Last`], so every `M` written anywhere in the frame (any schedule up to and
//! including `Update`) is captured the SAME update — no per-suite
//! `.after(the_writer)` ordering needed. The drain has its own [`MessageReader`]
//! cursor, so it observes every message regardless of other consumers draining the
//! buffer, and never double-counts across frames.

use std::marker::PhantomData;

use bevy::{
    app::{App, Last, Plugin},
    ecs::message::{Message, MessageReader},
    prelude::{ResMut, Resource},
};

/// Every `M` collected across the run, in emission order — read it in the test body
/// via [`probed`] (or `app.world().resource::<MessageProbe<M>>().seen()`).
#[derive(Resource)]
pub struct MessageProbe<M: Message> {
    /// The collected messages (private — read through [`Self::seen`]).
    seen: Vec<M>,
}

impl<M: Message> Default for MessageProbe<M> {
    fn default() -> Self {
        Self { seen: Vec::new() }
    }
}

impl<M: Message> MessageProbe<M> {
    /// The messages collected so far, in emission order.
    #[must_use]
    pub fn seen(&self) -> &[M] {
        &self.seen
    }

    /// Clears the collected messages — a mid-test reset, so a later read sees only
    /// post-reset emissions.
    pub fn clear(&mut self) {
        self.seen.clear();
    }
}

/// Drains newly-written `M`s into the [`MessageProbe<M>`] — registered in [`Last`]
/// by [`MessageProbePlugin`]; also registrable directly when a suite needs a custom
/// schedule/ordering.
pub fn drain_message_probe<M: Message + Clone>(
    mut reader: MessageReader<M>,
    mut probe: ResMut<MessageProbe<M>>,
) {
    probe.seen.extend(reader.read().cloned());
}

/// The collected `M`s, cloned out for assertion — empty if the probe was never
/// added (so a missing-plugin mistake reads as "zero messages", not a panic).
#[must_use]
pub fn probed<M: Message + Clone>(app: &App) -> Vec<M> {
    app.world()
        .get_resource::<MessageProbe<M>>()
        .map(|probe| probe.seen.clone())
        .unwrap_or_default()
}

/// Registers a [`MessageProbe<M>`] + its [`Last`]-schedule drain, and ensures the
/// `Messages<M>` buffer exists (`add_message` no-ops if a sim/app plugin already
/// registered it) — so the plugin is safe to add in any order.
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
