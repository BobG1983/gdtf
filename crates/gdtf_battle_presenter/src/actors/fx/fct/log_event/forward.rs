//! Forward played or live sim messages into combat log events.

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::{
        App, IntoScheduleConfigs, Message, MessageReader, MessageWriter, Res, SystemSet, Update,
        resource_exists,
    },
};
use gdtf_battle_sim::{battle::PlayerFaction, prelude::BattleInProgress, turn::TurnStarted};

use super::{event::CombatLogEvent, sight::PanelSight};
use crate::playback::Played;

/// System set for combat log forwarders.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatLogSystems {
    /// Forward played/live sources into [`CombatLogEvent`].
    Forward,
}

/// Something that can become a combat log event.
pub trait CombatLogSource: Message + Clone {
    /// Convert to a log event, or nothing when the screen never saw the act.
    fn to_event(&self, sight: &PanelSight) -> Option<CombatLogEvent>;
}

/// Forward `Played<S>` messages into combat log events.
pub fn forward_log_source<S: CombatLogSource>(
    mut source: MessageReader<Played<S>>,
    sight: PanelSight,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for message in source.read() {
        if let Some(event) = message.to_event(&sight) {
            events.write(event);
        }
    }
}

/// Forward live (non-played) messages into combat log events.
pub fn forward_live_log_source<S: CombatLogSource>(
    mut source: MessageReader<S>,
    sight: PanelSight,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for message in source.read() {
        if let Some(event) = message.to_event(&sight) {
            events.write(event);
        }
    }
}

/// Forward turn-started played messages with the player faction.
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

/// App extension to register combat log sources.
pub trait CombatLogSourceAppExt {
    /// Forward `Played<S>` into the combat log.
    fn add_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self;

    /// Forward live `S` into the combat log.
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
