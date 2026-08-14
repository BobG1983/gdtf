use bevy::prelude::*;
use gdtf_net_qa_transport::NetInbox;
use gdtf_qa_command::{
    catalogue::catalogue,
    dispatch::{CallQueues, IncomingCall, retest_waiting, route_call},
};
use gdtf_qa_protocol::message::{QaError, QaRequest, QaResponse};

use crate::dev::net_qa::{
    commands::{GAME_COMMANDS, game_host_name},
    facts::GameFactsParam,
};

pub(in crate::dev::net_qa) fn route_requests(
    inbox: Res<NetInbox>,
    facts: GameFactsParam,
    mut queues: CallQueues,
) {
    let command_facts = facts.sample();
    retest_waiting(GAME_COMMANDS, &command_facts, &mut queues);
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
                let call = IncomingCall::new(run.command, run.arguments, run.options, responder);
                route_call(GAME_COMMANDS, call, &command_facts, &mut queues);
            }
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }
}
