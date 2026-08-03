use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName, RunOptions},
    message::QaResponse,
};

use super::fake::FakeFacts;
use crate::{
    command::ErasedCommand,
    dispatch::{
        Admission, CommandInbox, admit, register_command_set, unavailable_reply, unknown_reply,
    },
};

pub fn fake_app(commands: &[&dyn ErasedCommand<FakeFacts>], facts: FakeFacts) -> App {
    let mut app = App::new();
    app.insert_resource(facts);
    register_command_set(&mut app, commands);
    app
}

#[must_use]
pub fn run_fake_command(
    app: &mut App,
    commands: &[&dyn ErasedCommand<FakeFacts>],
    name: &CommandName,
    arguments: &CommandArgsJson,
    options: &RunOptions,
) -> Receiver<QaResponse> {
    let (responder, answer) = Responder::channel();
    let facts = *app.world().resource::<FakeFacts>();
    match admit(commands, name, options, &facts) {
        Admission::Admit(_) => {
            app.world_mut().resource_mut::<CommandInbox>().admit(
                name.clone(),
                arguments.clone(),
                responder,
            );
        }
        Admission::Unavailable(refusal) => responder.reply(unavailable_reply(refusal)),
        Admission::Unknown(known) => responder.reply(unknown_reply(known)),
    }
    answer
}
