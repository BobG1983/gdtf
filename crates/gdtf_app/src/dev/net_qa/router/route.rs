use bevy::prelude::*;
use gdtf_net_qa_transport::NetInbox;
use gdtf_qa_command::{
    catalogue::catalogue,
    dispatch::{Admission, CommandInbox, admit, unavailable_reply, unknown_reply},
};
use gdtf_qa_protocol::message::{QaError, QaRequest, QaResponse};

use crate::dev::net_qa::{
    commands::{GAME_COMMANDS, game_host_name},
    facts::GameFactsParam,
};

pub(in crate::dev::net_qa) fn route_requests(
    inbox: Res<NetInbox>,
    facts: GameFactsParam,
    mut commands: ResMut<CommandInbox>,
) {
    let command_facts = facts.sample();
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            QaRequest::Catalogue => {
                responder.reply(QaResponse::Catalogue(catalogue(
                    game_host_name(),
                    GAME_COMMANDS,
                    &command_facts,
                )));
            }
            QaRequest::Run(run) => {
                match admit(GAME_COMMANDS, &run.command, &run.options, &command_facts) {
                    Admission::Admit(command) => {
                        commands.admit(command.name(), run.arguments, responder);
                    }
                    Admission::Unavailable(refusal) => responder.reply(unavailable_reply(refusal)),
                    Admission::Unknown(known) => responder.reply(unknown_reply(known)),
                }
            }
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }
}
