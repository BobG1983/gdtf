//! Fake commands and host facts for unit tests.

/// Command that publishes an unparseable schema.
pub mod broken;
/// Cell echo command.
pub mod cell;
/// Text echo command.
pub mod echo;
/// Host facts used by the fake command set.
pub mod facts;
/// Phase/status command.
pub mod phase;
/// Prebuilt command-set constants.
pub mod set;
/// Multi-frame settle command.
pub mod settle;
/// Command whose queue is never drained.
pub mod stall;
/// Duplicate of phase for name-collision tests.
pub mod twin;

pub use broken::FakeBrokenSchema;
pub use cell::{FakeCell, FakeCellArgs, FakeCellReply};
pub use echo::{FakeEcho, FakeEchoArgs, FakeEchoReply, FakeEchoText};
pub use facts::{FakeFacts, FakeLevel, FakeReady, fake_facts_loaded, fake_facts_unloaded};
pub use phase::{FakePhase, FakePhaseArgs, FakePhaseReply};
pub use set::{
    FAKE_COMMANDS, FAKE_COMMANDS_BROKEN_SCHEMA, FAKE_COMMANDS_DUPLICATED, FAKE_COMMANDS_GROWN,
    FAKE_COMMANDS_STALLED, fake_host_name,
};
pub use settle::{
    FakeSettle, FakeSettleArgs, FakeSettleCount, FakeSettleRaised, FakeSettleReply,
    FakeSettleSignal,
};
pub use stall::{FakeStall, FakeStallArgs, FakeStallLabel, FakeStallReply};
pub use twin::FakePhaseTwin;
