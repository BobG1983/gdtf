//! A fake host: one facts type, seven commands, and the slices they form.
//!
//! Deliberately NOT a miniature of the game. It is the smallest set that exercises every
//! branch the plumbing has: a command that is always available, one whose availability
//! turns on the facts, one that defers its answer, one that exists only to be ADDED (so
//! "adding a command moves no version" is a real test), one whose queue nobody drains (so
//! the pending-queue deadline has something to reap), one that duplicates another's name,
//! and one that publishes a schema document which is not JSON (so each conformance
//! assertion has something to fail on).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`facts`] — [`FakeFacts`] and its two sample values.
//! - [`phase`] — [`FakePhase`], always available.
//! - [`cell`] — [`FakeCell`], available only once the fake model is loaded.
//! - [`echo`] — [`FakeEcho`], the command the growth test adds.
//! - [`settle`] — [`FakeSettle`], the deferred one.
//! - [`stall`] — [`FakeStall`], whose queue nobody drains.
//! - [`twin`] — [`FakePhaseTwin`], a deliberate name collision.
//! - [`broken`] — [`FakeBrokenSchema`], a deliberately unparseable schema document.
//! - [`set`] — the slices.

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
