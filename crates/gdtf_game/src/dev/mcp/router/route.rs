use bevy::prelude::*;
use cobalt_mcp_host::{
    NetInbox,
    catalogue::catalogue,
    dispatch::{CallQueues, IncomingCall, retest_waiting, route_call},
};
use cobalt_mcp_protocol::message::{McpRequest, McpResponse, McpSessionError};

use crate::dev::mcp::{
    commands::{GAME_COMMANDS, game_host_name},
    facts::GameFactsParam,
};

pub(in crate::dev::mcp) fn route_requests(
    inbox: Res<NetInbox>,
    facts: GameFactsParam,
    mut queues: CallQueues,
) {
    let command_facts = facts.sample();
    retest_waiting(GAME_COMMANDS, &command_facts, &mut queues);
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            McpRequest::Catalogue => {
                responder.reply(McpResponse::Catalogue(catalogue(
                    game_host_name(),
                    GAME_COMMANDS,
                    &command_facts,
                )));
            }
            McpRequest::Run(run) => {
                let call = IncomingCall::new(run.command, run.arguments, run.options, responder);
                route_call(GAME_COMMANDS, call, &command_facts, &mut queues);
            }
            McpRequest::Hello(_) => {
                responder.reply(McpResponse::Error(McpSessionError::Malformed));
            }
        }
    }
}
