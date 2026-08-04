//! Pending contextual act targets and drain into sim requests.

use bevy::{ecs::message::Message, prelude::*};
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::{InputSystems, SelectedShooter, intent::dispatch_act_intents};

/// UI slot rank for ordering contextual act buttons.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SlotRank(u8);

impl SlotRank {
    /// Build from a zero-based rank.
    #[must_use]
    pub const fn new(rank: u8) -> Self {
        Self(rank)
    }
}

/// Trait implemented by each contextual act family.
pub trait ContextualAct: Send + Sync + 'static {
    /// Target type (entity, cell, etc.).
    type Target: Copy + PartialEq + core::fmt::Debug + Send + Sync + 'static;

    /// Sim request message written when the act fires.
    type Requested: Message;

    /// Build the sim request for `actor` and `target`.
    fn request(actor: Entity, target: Self::Target) -> Self::Requested;
}

/// Pending targets queued for contextual act `A`.
#[derive(Resource, Debug)]
pub struct PendingContextualIntents<A: ContextualAct>(Vec<A::Target>);

impl<A: ContextualAct> Default for PendingContextualIntents<A> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<A: ContextualAct> PendingContextualIntents<A> {
    /// Queue a target for later drain.
    pub fn push(&mut self, target: A::Target) {
        self.0.push(target);
    }

    pub(crate) fn drain(&mut self) -> Vec<A::Target> {
        core::mem::take(&mut self.0)
    }

    /// Whether no targets are pending.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// System set for draining contextual act queues.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextualActSystems {
    /// Drain pending targets into sim requests.
    Drain,
}

/// Write sim requests for each pending target when the playback gate is open.
pub fn drain_contextual_intents<A: ContextualAct>(
    gate: PlaybackGate,
    mut pending: ResMut<PendingContextualIntents<A>>,
    selected: Res<SelectedShooter>,
    mut requests: MessageWriter<A::Requested>,
) {
    let gate_open = gate.is_open();
    for target in pending.drain() {
        let Some(actor) = **selected else { continue };
        if !gate_open {
            continue;
        }
        requests.write(A::request(actor, target));
    }
}

/// App extension to register a contextual act family.
pub trait ContextualActAppExt {
    /// Init pending resource, message, and drain system for `A`.
    fn add_contextual_act<A: ContextualAct>(&mut self) -> &mut Self;
}

impl ContextualActAppExt for App {
    fn add_contextual_act<A: ContextualAct>(&mut self) -> &mut Self {
        self.add_message::<A::Requested>()
            .init_resource::<PendingContextualIntents<A>>()
            .add_systems(
                Update,
                drain_contextual_intents::<A>
                    .in_set(ContextualActSystems::Drain)
                    .run_if(resource_exists::<BattleInProgress>),
            )
    }
}

/// Place contextual act drains in the Gather set before intent dispatch.
pub fn configure_contextual_act_drains(app: &mut App) {
    app.configure_sets(
        Update,
        ContextualActSystems::Drain
            .in_set(InputSystems::Gather)
            .before(dispatch_act_intents),
    );
}
