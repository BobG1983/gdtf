use gdtf_qa_command::command::ErasedCommand;
use gdtf_qa_protocol::message::ServerNameNet;

use super::{
    capture::CaptureScreenshot,
    read::{
        AppPhase, BattleInspect, BattleOffers, BattleRoster, BattleSelection, BattleSightline,
        BattleTurn, BattleVisible, LogRead, PlaybackState, SettingsRead, UiFocus,
    },
};
use crate::dev::net_qa::{config::SERVER_NAME, facts::GameFacts};

pub(in crate::dev::net_qa) const GAME_COMMANDS: &[&dyn ErasedCommand<GameFacts>] = &[
    &AppPhase,
    &CaptureScreenshot,
    &SettingsRead,
    &UiFocus,
    &PlaybackState,
    &BattleRoster,
    &BattleTurn,
    &BattleSelection,
    &BattleOffers,
    &BattleInspect,
    &BattleSightline,
    &BattleVisible,
    &LogRead,
];

#[must_use]
pub(in crate::dev::net_qa) fn game_host_name() -> ServerNameNet {
    ServerNameNet::new(SERVER_NAME.to_owned())
}
