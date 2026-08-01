//! [`route_requests`] — the ONE drain of the [`NetInbox`] (GTW-736, GTW-942, GTW-943).

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

/// Drains the inbox and dispatches every buffered request (see the module doc).
///
/// The command facts are sampled ONCE, before the drain, not per request: two calls in one
/// frame that disagreed about the world would be answered from two worlds that never
/// existed together.
pub(in crate::dev::net_qa) fn route_requests(
    inbox: Res<NetInbox>,
    facts: GameFactsParam,
    mut commands: ResMut<CommandInbox>,
) {
    let command_facts = facts.sample();
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            // A catalogue IS the frame's facts read through every command's own predicate,
            // so there is nothing to defer: it is answered right here.
            QaRequest::Catalogue => {
                responder.reply(QaResponse::Catalogue(catalogue(
                    game_host_name(),
                    GAME_COMMANDS,
                    &command_facts,
                )));
            }
            // A `Run` is RESOLVED here and handled elsewhere: `admit` scans this host's own
            // slice, so an unknown name and an unavailable command are both answered at route
            // time with the correction a caller needs, and only an admitted call is parked for
            // its command's decode step. The name never selects a function pointer — there is
            // no `match` on it anywhere.
            QaRequest::Run(run) => {
                match admit(GAME_COMMANDS, &run.command, &run.options, &command_facts) {
                    Admission::Admit(command) => {
                        commands.admit(command.name(), run.arguments, responder);
                    }
                    Admission::Unavailable(refusal) => responder.reply(unavailable_reply(refusal)),
                    Admission::Unknown(known) => responder.reply(unknown_reply(known)),
                }
            }
            // Unreachable in practice: the LISTENER THREAD negotiates a `Hello` against the
            // host's `hello_facts()` and never forwards it (GTW-940). One that arrived anyway
            // got past the only code that answers a handshake, so the frame is wrong for this
            // connection. Listed exhaustively (no wildcard arm) so a future request variant
            // fails to compile here rather than being silently swallowed by a catch-all.
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }
}
