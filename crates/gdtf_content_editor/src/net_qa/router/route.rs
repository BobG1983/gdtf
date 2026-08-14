//! Route net-QA requests. Version negotiation is handled by the listener thread.
use bevy::prelude::*;
use gdtf_net_qa_transport::NetInbox;
use gdtf_qa_command::{
    catalogue::catalogue,
    dispatch::{CallQueues, IncomingCall, retest_waiting, route_call},
};
use gdtf_qa_protocol::message::{QaError, QaRequest, QaResponse};

use crate::net_qa::{commands::EDITOR_COMMANDS, config::editor_host_name, facts::EditorFactsParam};

pub(in crate::net_qa) fn route_editor_requests(
    inbox: Res<NetInbox>,
    facts: EditorFactsParam,
    mut queues: CallQueues,
) {
    let command_facts = facts.sample();
    retest_waiting(EDITOR_COMMANDS, &command_facts, &mut queues);
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            QaRequest::Catalogue => {
                responder.reply(QaResponse::Catalogue(catalogue(
                    editor_host_name(),
                    EDITOR_COMMANDS,
                    &command_facts,
                )));
            }
            QaRequest::Run(run) => {
                let call = IncomingCall::new(run.command, run.arguments, run.options, responder);
                route_call(EDITOR_COMMANDS, call, &command_facts, &mut queues);
            }
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }
}
