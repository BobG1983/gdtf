//! Read played consequence signals and spawn stacked floating text.

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::{
        App, Commands, IntoScheduleConfigs, MessageReader, Query, Res, SystemSet, Update,
        resource_exists,
    },
};
use gdtf_battle_sim::prelude::{BattleInProgress, Position};

use super::{
    super::FxTuning,
    pop::{ConsequenceFct, PopAnchor},
    slot_allocator::FctSlotAllocator,
    text::{FctDrift, FctSlot, animate_floating_text, spawn_floating_text},
};
use crate::{
    PresenterSystems,
    playback::{DrawnPosition, Played},
};

type AnchorData = (&'static Position, Option<&'static DrawnPosition>);

/// System set for consequence FCT readers.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsequenceFctSystems {
    /// Read played signals and spawn pops.
    Read,
}

/// Spawn floating combat text for each played consequence of type `C`.
pub fn read_consequence_fct<C: ConsequenceFct>(
    mut commands: Commands,
    mut signals: MessageReader<Played<C::Signal>>,
    anchors: Query<AnchorData>,
    allocator: FctSlotAllocator,
    tuning: Res<FxTuning>,
) {
    for signal in signals.read() {
        let pop = C::classify(&**signal);
        let at = match pop.anchor() {
            PopAnchor::Carried(at) => at,
            PopAnchor::GangerPosition(entity) => {
                let Ok((position, drawn)) = anchors.get(entity) else {
                    continue;
                };
                drawn.map_or(**position, |drawn| *drawn.position())
            }
        };
        let slot = allocator.next_slot(at);
        spawn_floating_text(
            &mut commands,
            pop.label(),
            FctSlot::new(at, slot),
            FctDrift::from_tuning(&tuning),
        );
    }
}

/// Configure the consequence FCT system set.
pub fn register_consequence_fct_core(app: &mut App) {
    app.configure_sets(
        Update,
        ConsequenceFctSystems::Read
            .in_set(PresenterSystems::Overlay)
            .after(animate_floating_text),
    );
}

/// App extension to register a consequence FCT family.
pub trait ConsequenceFctAppExt {
    /// Add a reader for consequence family `C`.
    fn add_consequence_fct<C: ConsequenceFct>(&mut self) -> &mut Self;
}

impl ConsequenceFctAppExt for App {
    fn add_consequence_fct<C: ConsequenceFct>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            read_consequence_fct::<C>
                .in_set(ConsequenceFctSystems::Read)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<Messages<Played<C::Signal>>>)
                        .and_then(resource_exists::<Messages<C::Signal>>)
                        .and_then(resource_exists::<FxTuning>),
                ),
        );
        self
    }
}
