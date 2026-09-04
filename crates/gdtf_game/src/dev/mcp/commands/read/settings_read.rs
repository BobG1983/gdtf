use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use serde::{Deserialize, Serialize};

use crate::{
    dev::mcp::{facts::GameFacts, wire::shell::SoundNet},
    states::running::options::settings::GameSettings,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SettingsReadArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SettingsReadReply {
    sound: SoundNet,
}

pub(crate) struct SettingsRead;

impl McpCommand for SettingsRead {
    type Args = SettingsReadArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = SettingsReadReply;

    const NAME: CommandName = CommandName::from_static("settings.read");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the Options values the player can change — today that is whether sound is on. \
         The settings resource exists from the moment the app is built, so this answers \
         outside a battle and needs no screen to be open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_settings_read.after(McpCommandSystems::Claim));
    }
}

fn handle_settings_read(
    settings: Res<GameSettings>,
    mut queue: ResMut<PendingQueue<CommandCall<SettingsRead>>>,
) {
    if queue.is_empty() {
        return;
    }
    let sound = SoundNet::new(*settings.sound);
    for (_args, responder) in take_calls::<SettingsRead>(&mut queue) {
        responder.answer(&SettingsReadReply { sound });
    }
}
