//! Route net-QA requests. Version negotiation is handled by the listener thread.
use bevy::prelude::*;
use gdtf_net_qa_transport::NetInbox;
use gdtf_qa_protocol::{
    command::{CommandCatalogue, CommandOutcome},
    message::{QaError, QaRequest, QaResponse, ServerNameNet},
};

use crate::net_qa::config::EDITOR_QA_SERVER_NAME;

pub(in crate::net_qa) fn route_editor_requests(inbox: Res<NetInbox>) {
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            QaRequest::Catalogue => {
                responder.reply(QaResponse::Catalogue(CommandCatalogue::new(
                    ServerNameNet::new(EDITOR_QA_SERVER_NAME.to_owned()),
                    Vec::new(),
                )));
            }
            QaRequest::Run(_) => {
                responder.reply(QaResponse::Outcome(CommandOutcome::Unknown {
                    known: Vec::new(),
                }));
            }
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }
}
