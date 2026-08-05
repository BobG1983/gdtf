//! Game command set conformance checks for tests.

use gdtf_net_qa_transport::DEFAULT_IO_TIMEOUT;
use gdtf_qa_protocol::command::CommandName;

use super::set::GAME_COMMANDS;

/// Assert the registered game commands are unique, publish shapes, and expire in time.
///
/// # Panics
///
/// Panics when a command's deferral budget would outlive the socket's own wait.
pub fn assert_game_command_set_is_conformant() {
    gdtf_qa_command::test_support::assert_unique_names(GAME_COMMANDS);
    gdtf_qa_command::test_support::assert_schemas_parse(GAME_COMMANDS);
    gdtf_qa_command::test_support::assert_shape_names_agree(GAME_COMMANDS);
    for command in GAME_COMMANDS {
        let budget = command.deferred_budget();
        assert!(
            *budget < *DEFAULT_IO_TIMEOUT,
            "`{}`'s deferral budget ({:?}) must expire strictly before the socket timeout ({:?})",
            command.name().as_str(),
            *budget,
            *DEFAULT_IO_TIMEOUT
        );
    }
}

/// Names of all registered game commands.
#[must_use]
pub fn game_command_names() -> Vec<CommandName> {
    GAME_COMMANDS.iter().map(|command| command.name()).collect()
}
