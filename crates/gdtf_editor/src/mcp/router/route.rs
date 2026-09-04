//! Route MCP requests. Version negotiation is handled by the listener thread.
use bevy::prelude::*;
use cobalt_mcp_command::{
    catalogue::catalogue,
    dispatch::{CallQueues, IncomingCall, retest_waiting, route_call},
};
use cobalt_mcp_protocol::message::{QaError, QaRequest, QaResponse};
use cobalt_mcp_transport::NetInbox;

use crate::mcp::{commands::EDITOR_COMMANDS, config::editor_host_name, facts::EditorFactsParam};

pub(in crate::mcp) fn route_editor_requests(
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
