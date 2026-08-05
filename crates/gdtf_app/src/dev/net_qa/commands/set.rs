use gdtf_qa_command::command::ErasedCommand;
use gdtf_qa_protocol::message::ServerNameNet;

use super::{
    capture::CaptureScreenshot,
    read::{AppPhase, PlaybackState, SettingsRead, UiFocus},
};
use crate::dev::net_qa::{config::SERVER_NAME, facts::GameFacts};

pub(in crate::dev::net_qa) const GAME_COMMANDS: &[&dyn ErasedCommand<GameFacts>] = &[
    &AppPhase,
    &CaptureScreenshot,
    &SettingsRead,
    &UiFocus,
    &PlaybackState,
];

#[must_use]
pub(in crate::dev::net_qa) fn game_host_name() -> ServerNameNet {
    ServerNameNet::new(SERVER_NAME.to_owned())
}
