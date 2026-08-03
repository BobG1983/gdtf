use gdtf_qa_protocol::message::ServerNameNet;

use super::{
    broken::FakeBrokenSchema, cell::FakeCell, echo::FakeEcho, facts::FakeFacts, phase::FakePhase,
    settle::FakeSettle, stall::FakeStall, twin::FakePhaseTwin,
};
use crate::command::ErasedCommand;

#[must_use]
pub fn fake_host_name() -> ServerNameNet {
    ServerNameNet::new("fake".to_owned())
}

pub const FAKE_COMMANDS: &[&dyn ErasedCommand<FakeFacts>] = &[&FakePhase, &FakeCell, &FakeSettle];

pub const FAKE_COMMANDS_GROWN: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeCell, &FakeSettle, &FakeEcho];

pub const FAKE_COMMANDS_DUPLICATED: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakePhaseTwin];

pub const FAKE_COMMANDS_BROKEN_SCHEMA: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeBrokenSchema];

pub const FAKE_COMMANDS_STALLED: &[&dyn ErasedCommand<FakeFacts>] = &[&FakeStall];
