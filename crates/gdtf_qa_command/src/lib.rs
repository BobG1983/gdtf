//! The shared QA command plumbing (GTW-941, phase A3 of the GTW-938 protocol rewrite).
//!
//! A host — the game, the content editor — owns ONE list of typed commands. This crate
//! turns that list into everything a QA client sees: the published
//! [`CommandCatalogue`](gdtf_qa_protocol::command::CommandCatalogue), the decision to
//! admit or refuse one incoming `Run`, the decode of its JSON arguments into the command's
//! own argument type, and the typed handler call that answers it. Nothing here knows about
//! a battle, an editor draft, or a socket.
//!
//! ## The property this exists for
//!
//! Adding a command adds a file and one line in a host's list. It moves no protocol
//! version, adds no wire variant, and changes no courier — because a command is DATA
//! carried inside two frozen envelope variants, not a variant of its own. The
//! `adding_a_command_moves_no_version` test in `tests/command_set/` proves it rather than
//! asserting it in prose.
//!
//! ## Two traits, and why
//!
//! [`QaCommand`](command::QaCommand) is what an implementer writes: two associated types
//! (whose JSON Schemas are DERIVED, never hand-written), a name, a summary, a pure
//! availability predicate, and the registration of an ordinary Bevy system. It is not
//! object-safe — every interesting method mentions `Self::Args` — so it cannot be the
//! thing a catalogue walks.
//!
//! [`ErasedCommand<F>`](command::ErasedCommand) is the object-safe projection: every
//! method returns a concrete type, so `dyn ErasedCommand<F>` exists and a host's list can
//! be one slice. Outside `test_support` it has exactly ONE implementation, the blanket
//! `impl<C: QaCommand> ErasedCommand<C::Facts> for C`, and must never be implemented by
//! hand. The single hand-written impl in the repo is the `test_support` fixture
//! `FakeBrokenSchema`, which exists only so `assert_schemas_parse` can be watched catching
//! the very thing this warning is about.
//!
//! ## Compiler-enforced properties (deliberate, recorded here)
//!
//! - **A host's list cannot hold another host's command.** The blanket impl ties a
//!   command's `Facts` to the `F` of the trait object, so an entry whose
//!   [`QaCommand::Facts`](command::QaCommand::Facts) is not the slice's `F` fails to
//!   coerce AT THE SLICE LITERAL, before anything runs. A `compile_fail` doc test on
//!   [`ErasedCommand`](command::ErasedCommand) pins it; the positive counterpart is the
//!   `test_support` fake set, which is a slice of exactly that shape and is compiled on
//!   every run of the suite.
//! - **A name never selects a function pointer.** [`admit()`](dispatch::admit()) is a linear
//!   scan over the host's own slice of typed items — no wildcard, no string dispatch
//!   table, no `match` on a name anywhere in this crate.
//!
//! ## What the wiring gives you, which the type system does not
//!
//! On the path [`register_command`](dispatch::register_command) builds, a handler receives
//! `C::Args` and a [`CommandResponder<C>`](dispatch::CommandResponder) that accepts only
//! `&C::Reply`: [`claim_calls`](dispatch::claim_calls) does the decode, and the round-trip
//! test in `tests/command_set/` shows the handler never touching JSON. That is a property of
//! this wiring, NOT a compiler guarantee — [`CommandInbox::take_for`](dispatch::CommandInbox)
//! hands out `(CommandArgsJson, Responder)` pairs and
//! `PendingQueue::drain_ready` is public, so a host that reaches past
//! `register_command` can still hold both. Write a command against
//! [`QaCommand`](command::QaCommand) and the raw forms stay out of reach in practice.
//!
//! ## The riders are STUBS in this crate
//!
//! [`RunOptions`](gdtf_qa_protocol::command::RunOptions) carries `await_ready` and
//! `capture`. Neither is built here. A call carrying either is answered
//! [`Unavailable`](gdtf_qa_protocol::command::CommandOutcome::Unavailable) with code
//! [`NotBuilt`](gdtf_qa_protocol::command::UnavailableCode::NotBuilt) — an honest refusal
//! rather than running the command and silently dropping the rider the caller asked for.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`command`] — the two traits and the schema derivation.
//! - [`dispatch`] — admission, the inbox, the typed queue, the decode step, the typed
//!   responder, deferred replies, registration and the system sets.
//! - [`catalogue`] — building the published catalogue from a host's slice.
//! - `test_support` (feature `test-support`) — the fake command set, a bare-`App` harness,
//!   and the two per-host conformance assertions.

pub mod catalogue;
pub mod command;
pub mod dispatch;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
