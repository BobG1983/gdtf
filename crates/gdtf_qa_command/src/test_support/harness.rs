//! A bare `App` wired with a fake command set, and the router stand-in that drives one
//! call through it.

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

/// A BARE `App` — no `DefaultPlugins`, no `MinimalPlugins` — wired with a fake command set
/// and a facts sample.
///
/// Bare on purpose: the point of the suite is that this plumbing needs nothing from a host
/// to work, so anything the app has beyond the schedules `App::new()` brings would weaken
/// the claim.
pub fn fake_app(commands: &[&dyn ErasedCommand<FakeFacts>], facts: FakeFacts) -> App {
    let mut app = App::new();
    // No `configure_sets` here on purpose: `register_command` chains Route before Claim
    // itself, so every case below is running the ordering a real host gets, not one the
    // harness arranged for it.
    app.insert_resource(facts);
    register_command_set(&mut app, commands);
    app
}

/// Drive one call into the app exactly the way a host's router does, and hand back the
/// channel its answer will arrive on.
///
/// This is the router stand-in, and it is deliberately the same four lines a real host
/// writes: sample the facts, [`admit()`](crate::dispatch::admit()), and either park the call in the
/// [`CommandInbox`] or answer the refusal at route time. A call that is not admissible is
/// REJECTED here and never queued — the pending-queue deadline is a few frames, far shorter
/// than any window a gated call would have to wait.
///
/// The app is not advanced; the caller decides how many frames to run.
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
