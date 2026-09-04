//! Prebuilt fake command sets for different test scenarios.

use cobalt_mcp_protocol::message::ServerNameNet;

use super::{
    broken::FakeBrokenSchema, echo::FakeEcho, facts::FakeFacts, phase::FakePhase, point::FakePoint,
    settle::FakeSettle, stall::FakeStall, twin::FakePhaseTwin,
};
use crate::command::ErasedCommand;

/// Host name used by the fake catalogue.
#[must_use]
pub fn fake_host_name() -> ServerNameNet {
    ServerNameNet::new("fake".to_owned())
}

/// Default set: phase, point, settle.
pub const FAKE_COMMANDS: &[&dyn ErasedCommand<FakeFacts>] = &[&FakePhase, &FakePoint, &FakeSettle];

/// Default set plus echo.
pub const FAKE_COMMANDS_GROWN: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakePoint, &FakeSettle, &FakeEcho];

/// Two commands that claim the same name.
pub const FAKE_COMMANDS_DUPLICATED: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakePhaseTwin];

/// Set that includes a broken schema command.
pub const FAKE_COMMANDS_BROKEN_SCHEMA: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeBrokenSchema];

/// Set whose only command never drains its queue.
pub const FAKE_COMMANDS_STALLED: &[&dyn ErasedCommand<FakeFacts>] = &[&FakeStall];
