use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_presenter::playback::PlaybackGate;
use serde::{Deserialize, Serialize};

use crate::dev::mcp::{facts::GameFacts, wire::shell::CaughtUpNet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlaybackStateArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PlaybackStateReply {
    caught_up: CaughtUpNet,
}

pub(crate) struct PlaybackState;

impl McpCommand for PlaybackState {
    type Args = PlaybackStateArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = PlaybackStateReply;

    const NAME: CommandName = CommandName::from_static("playback.state");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read whether the screen has caught up with the act log — the same gate that decides \
         when player input may proceed. Outside a battle there is nothing to catch up with, so \
         it answers true; poll it to wait for an act to finish playing.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_playback_state.after(McpCommandSystems::Claim),
        );
    }
}

fn handle_playback_state(
    gate: PlaybackGate,
    mut queue: ResMut<PendingQueue<CommandCall<PlaybackState>>>,
) {
    if queue.is_empty() {
        return;
    }
    let caught_up = CaughtUpNet::new(gate.is_open());
    for (_args, responder) in take_calls::<PlaybackState>(&mut queue) {
        responder.answer(&PlaybackStateReply { caught_up });
    }
}
