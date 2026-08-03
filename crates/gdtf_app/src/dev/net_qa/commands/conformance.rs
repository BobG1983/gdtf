//! Game command set conformance checks for tests.

use gdtf_qa_protocol::command::CommandName;

use super::set::GAME_COMMANDS;

/// Assert the registered game commands have unique names and parseable schemas.
pub fn assert_game_command_set_is_conformant() {
    gdtf_qa_command::test_support::assert_unique_names(GAME_COMMANDS);
    gdtf_qa_command::test_support::assert_schemas_parse(GAME_COMMANDS);
}

/// Names of all registered game commands.
#[must_use]
pub fn game_command_names() -> Vec<CommandName> {
    GAME_COMMANDS.iter().map(|command| command.name()).collect()
}
