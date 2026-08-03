//! Prebuilt fake command sets for different test scenarios.

use gdtf_qa_protocol::message::ServerNameNet;

use super::{
    broken::FakeBrokenSchema, cell::FakeCell, echo::FakeEcho, facts::FakeFacts, phase::FakePhase,
    settle::FakeSettle, stall::FakeStall, twin::FakePhaseTwin,
};
use crate::command::ErasedCommand;

/// Host name used by the fake catalogue.
#[must_use]
pub fn fake_host_name() -> ServerNameNet {
    ServerNameNet::new("fake".to_owned())
}

/// Default set: phase, cell, settle.
pub const FAKE_COMMANDS: &[&dyn ErasedCommand<FakeFacts>] = &[&FakePhase, &FakeCell, &FakeSettle];

/// Default set plus echo.
pub const FAKE_COMMANDS_GROWN: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeCell, &FakeSettle, &FakeEcho];

/// Two commands that claim the same name.
pub const FAKE_COMMANDS_DUPLICATED: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakePhaseTwin];

/// Set that includes a broken schema command.
pub const FAKE_COMMANDS_BROKEN_SCHEMA: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeBrokenSchema];

/// Set whose only command never drains its queue.
pub const FAKE_COMMANDS_STALLED: &[&dyn ErasedCommand<FakeFacts>] = &[&FakeStall];
