//! Mini Bevy host for running fake commands in tests.

use std::sync::mpsc::Receiver;

use bevy::{ecs::system::SystemState, prelude::*};
use cobalt_mcp_protocol::{
    command::{CommandArgsRon, CommandName, RunOptions},
    message::QaResponse,
};
use cobalt_mcp_transport::Responder;

use super::fake::FakeFacts;
use crate::{
    command::ErasedCommand,
    dispatch::{
        CallQueues, IncomingCall, QaCommandSystems, register_command_set, retest_waiting,
        route_call,
    },
};

/// The command set the fake host re-tests its held calls against.
#[derive(Resource, Deref)]
pub struct FakeCommandSet(&'static [&'static dyn ErasedCommand<FakeFacts>]);

impl FakeCommandSet {
    /// Wrap the set a fake host was built with.
    #[must_use]
    pub const fn new(commands: &'static [&'static dyn ErasedCommand<FakeFacts>]) -> Self {
        Self(commands)
    }
}

/// Build an app with the given command set and facts resource.
pub fn fake_app(
    commands: &'static [&'static dyn ErasedCommand<FakeFacts>],
    facts: FakeFacts,
) -> App {
    let mut app = App::new();
    app.insert_resource(facts);
    app.insert_resource(FakeCommandSet::new(commands));
    register_command_set(&mut app, commands);
    app.add_systems(Update, retest_fake_waiting.in_set(QaCommandSystems::Route));
    app
}

fn retest_fake_waiting(
    facts: Res<FakeFacts>,
    commands: Res<FakeCommandSet>,
    mut queues: CallQueues,
) {
    retest_waiting(**commands, &facts, &mut queues);
}

/// Route a command call into the app and return the reply channel.
#[must_use]
pub fn run_fake_command(
    app: &mut App,
    commands: &[&dyn ErasedCommand<FakeFacts>],
    name: &CommandName,
    arguments: &CommandArgsRon,
    options: &RunOptions,
) -> Receiver<QaResponse> {
    let (responder, answer) = Responder::channel();
    let facts = *app.world().resource::<FakeFacts>();
    let call = IncomingCall::new(name.clone(), arguments.clone(), options.clone(), responder);
    let mut state: SystemState<CallQueues> = SystemState::new(app.world_mut());
    if let Ok(mut queues) = state.get_mut(app.world_mut()) {
        route_call(commands, call, &facts, &mut queues);
    }
    answer
}
