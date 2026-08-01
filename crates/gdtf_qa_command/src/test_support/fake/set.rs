//! The fake host's slices — the ONE list shape a real host uses, three times over.

use gdtf_qa_protocol::message::ServerNameNet;

use super::{
    broken::FakeBrokenSchema, cell::FakeCell, echo::FakeEcho, facts::FakeFacts, phase::FakePhase,
    settle::FakeSettle, stall::FakeStall, twin::FakePhaseTwin,
};
use crate::command::ErasedCommand;

/// The fake host's name, as a catalogue reports it.
#[must_use]
pub fn fake_host_name() -> ServerNameNet {
    ServerNameNet::new("fake".to_owned())
}

/// The fake host's command set: the two the suite proves the abstraction on, plus the
/// deferred one.
///
/// Each entry is a reference to a unit struct, promoted to `'static` in const position —
/// the same shape a real host's `GAME_COMMANDS` / `EDITOR_COMMANDS` takes. The compiler
/// enforces the coupling: an entry whose `Facts` is not [`FakeFacts`] fails to coerce here.
pub const FAKE_COMMANDS: &[&dyn ErasedCommand<FakeFacts>] = &[&FakePhase, &FakeCell, &FakeSettle];

/// The same set with a THIRD read command added.
///
/// The growth test builds a catalogue from both slices and compares: one more entry, the
/// new command resolves and runs, and the protocol version is untouched — because a
/// command is data inside two frozen envelope variants, not a variant of its own.
pub const FAKE_COMMANDS_GROWN: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeCell, &FakeSettle, &FakeEcho];

/// A deliberately BROKEN set — two entries claiming one name.
///
/// Never registered and never run: it exists so `assert_unique_names` can be shown to fail
/// on a real slice rather than only on a hand-built row.
pub const FAKE_COMMANDS_DUPLICATED: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakePhaseTwin];

/// A second deliberately BROKEN set — one entry publishing a schema document that is not
/// JSON.
///
/// Never registered and never run, for the same reason as the duplicated set: it exists so
/// `assert_schemas_parse` can be shown to fail on a real slice. It needs the hand-written
/// [`FakeBrokenSchema`] because a `schemars`-derived schema cannot be made unparseable —
/// which is the whole reason the assertion is worth having.
pub const FAKE_COMMANDS_BROKEN_SCHEMA: &[&dyn ErasedCommand<FakeFacts>] =
    &[&FakePhase, &FakeBrokenSchema];

/// The set the stalled-call test registers: one command whose queue nobody drains.
///
/// Kept out of [`FAKE_COMMANDS`] on purpose — a command that never answers would make every
/// other case wait on a deadline it does not care about.
pub const FAKE_COMMANDS_STALLED: &[&dyn ErasedCommand<FakeFacts>] = &[&FakeStall];
