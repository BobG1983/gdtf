//! Test-only helpers: assertions, fake commands, and a mini host app.

/// Check unique names and parseable shapes on a command set.
pub mod assert;
/// Fake commands and host facts for unit tests.
pub mod fake;
/// Build a Bevy app and run a fake command through admission.
pub mod harness;

pub use assert::{
    CommandRow, NameCheck, SchemaCheck, SchemaSide, ShapeNameCheck, assert_schemas_parse,
    assert_shape_names_agree, assert_unique_names, check_schemas_parse, check_shape_names_agree,
    check_unique_names, command_rows,
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
