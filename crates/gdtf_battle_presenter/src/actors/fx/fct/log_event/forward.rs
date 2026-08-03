use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::{
        App, IntoScheduleConfigs, Message, MessageReader, MessageWriter, Query, Res, SystemSet,
        Update, resource_exists,
    },
};
use gdtf_battle_sim::{
    battle::PlayerFaction, ganger::GangerName, prelude::BattleInProgress, turn::TurnStarted,
};

use super::event::{CombatLogEvent, LogName};
use crate::playback::Played;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatLogSystems {
        Forward,
}

pub trait CombatLogSource: Message + Clone {
        fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent>;
}

pub(super) fn name_of(entity: bevy::prelude::Entity, names: &Query<&GangerName>) -> LogName {
    names
        .get(entity)
        .map_or_else(|_| LogName::new("Someone"), LogName::from_ganger)
}

pub fn forward_log_source<S: CombatLogSource>(
    mut source: MessageReader<Played<S>>,
    names: Query<&GangerName>,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for message in source.read() {
        if let Some(event) = message.to_event(&names) {
            events.write(event);
        }
    }
}

pub fn forward_live_log_source<S: CombatLogSource>(
    mut source: MessageReader<S>,
    names: Query<&GangerName>,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for message in source.read() {
        if let Some(event) = message.to_event(&names) {
            events.write(event);
        }
    }
}

pub fn forward_turn_started(
    mut turns: MessageReader<Played<TurnStarted>>,
    player: Option<Res<PlayerFaction>>,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for turn in turns.read() {
        let Some(player) = player.as_deref() else {
            continue;
        };
        events.write(CombatLogEvent::TurnStarted {
            now_active: turn.now_active,
            player:     *player,
        });
    }
}

pub trait CombatLogSourceAppExt {
                                                fn add_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self;

            fn add_live_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self;
}

impl CombatLogSourceAppExt for App {
    fn add_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            forward_log_source::<S>
                .in_set(CombatLogSystems::Forward)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<S>>),
                ),
        );
        self
    }

    fn add_live_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            forward_live_log_source::<S>
                .in_set(CombatLogSystems::Forward)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<S>>),
                ),
        );
        self
    }
}
