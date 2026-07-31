//! The fake command set, the bare-`App` harness, and the two per-host conformance
//! assertions (feature `test-support`).
//!
//! Two things live here rather than in a test file. The FAKES, because they let this
//! crate's own suite prove the abstraction on a bare `App` before any real command exists —
//! nothing about the game or the editor is needed to show that admit → claim → decode →
//! handle → reply works. And the ASSERTIONS, because they have to run over a HOST's slice:
//! "no two commands share a name" and "every published schema is parseable JSON" are
//! facts about the game's list and the editor's list, so each host calls them from its own
//! suite.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`fake`] — the fake facts type, the fake commands, and the slices they form.
//! - [`assert`] — the two conformance assertions and the checks underneath them.
//! - [`harness`] — a bare `App` wired with a fake set, and the router stand-in that drives
//!   one call through it.

pub mod assert;
pub mod fake;
pub mod harness;

pub use assert::{
    CommandRow, NameCheck, SchemaCheck, SchemaSide, assert_schemas_parse, assert_unique_names,
    check_schemas_parse, check_unique_names, command_rows,
};
pub use fake::{
    FAKE_COMMANDS, FAKE_COMMANDS_BROKEN_SCHEMA, FAKE_COMMANDS_DUPLICATED, FAKE_COMMANDS_GROWN,
    FAKE_COMMANDS_STALLED, FakeBrokenSchema, FakeCell, FakeCellArgs, FakeCellReply, FakeEcho,
    FakeEchoArgs, FakeEchoReply, FakeEchoText, FakeFacts, FakeLevel, FakePhase, FakePhaseArgs,
    FakePhaseReply, FakePhaseTwin, FakeReady, FakeSettle, FakeSettleArgs, FakeSettleCount,
    FakeSettleRaised, FakeSettleReply, FakeSettleSignal, FakeStall, FakeStallArgs, FakeStallLabel,
    FakeStallReply, fake_facts_loaded, fake_facts_unloaded, fake_host_name,
};
pub use harness::{fake_app, run_fake_command};

#[cfg(test)]
mod test;
