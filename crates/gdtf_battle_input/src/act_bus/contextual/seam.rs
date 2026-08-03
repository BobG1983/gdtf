use bevy::{ecs::message::Message, prelude::*};
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::{InputSystems, SelectedShooter, intent::dispatch_act_intents};

#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SlotRank(u8);

impl SlotRank {
        #[must_use]
    pub const fn new(rank: u8) -> Self {
        Self(rank)
    }
}

pub trait ContextualAct: Send + Sync + 'static {
                        type Target: Copy + PartialEq + core::fmt::Debug + Send + Sync + 'static;

            type Requested: Message;

                        fn request(actor: Entity, target: Self::Target) -> Self::Requested;
}

#[derive(Resource, Debug)]
pub struct PendingContextualIntents<A: ContextualAct>(Vec<A::Target>);

impl<A: ContextualAct> Default for PendingContextualIntents<A> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<A: ContextualAct> PendingContextualIntents<A> {
                            pub fn push(&mut self, target: A::Target) {
        self.0.push(target);
    }

                        pub(crate) fn drain(&mut self) -> Vec<A::Target> {
        core::mem::take(&mut self.0)
    }

        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextualActSystems {
        Drain,
}

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

pub trait ContextualActAppExt {
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

pub fn configure_contextual_act_drains(app: &mut App) {
    app.configure_sets(
        Update,
        ContextualActSystems::Drain
            .in_set(InputSystems::Gather)
            .before(dispatch_act_intents),
    );
}
