//! The GAME host's half of the two per-host conformance assertions (GTW-942).
//!
//! `gdtf_qa_command`'s `assert_unique_names` / `assert_schemas_parse` are facts about ONE
//! host's slice, so they have to be called with that slice — and no test inside
//! `gdtf_qa_command` can see `GAME_COMMANDS`.
//!
//! They are called from HERE rather than from the slice being handed out, because the
//! slice's type names `GameFacts`, which names `AppPhaseNet`: exporting the constant would
//! drag this host's whole internal vocabulary onto the crate's public surface for the
//! benefit of two assertions. The suite gets what it actually needs — the assertions run
//! over the real slice, and the real names that slice publishes — and the vocabulary stays
//! inside the module that owns it.

use gdtf_qa_protocol::command::CommandName;

use super::set::GAME_COMMANDS;

/// Run both per-host conformance assertions over the REAL `GAME_COMMANDS` slice.
///
/// # Panics
///
/// Fails the calling test when two commands claim one name, or when any published schema
/// document is not parseable JSON.
pub fn assert_game_command_set_is_conformant() {
    gdtf_qa_command::test_support::assert_unique_names(GAME_COMMANDS);
    gdtf_qa_command::test_support::assert_schemas_parse(GAME_COMMANDS);
}

/// Every name the GAME host's command set publishes, in catalogue order.
///
/// What a suite asserts the SET against without reaching the slice itself — a command
/// added to (or dropped from) `GAME_COMMANDS` moves this, and nothing else has to be
/// edited for the check to notice.
#[must_use]
pub fn game_command_names() -> Vec<CommandName> {
    GAME_COMMANDS.iter().map(|command| command.name()).collect()
}
