//! "adding a command moves no version" is a real test), one whose queue nobody drains (so
pub mod broken;
pub mod cell;
pub mod echo;
pub mod facts;
pub mod phase;
pub mod set;
pub mod settle;
pub mod stall;
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
