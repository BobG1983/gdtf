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
