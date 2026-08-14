//! Editor command set conformance checks for tests.

use gdtf_qa_protocol::{command::CommandName, timeouts::DEFAULT_REPLY_TIMEOUT};

use super::set::EDITOR_COMMANDS;

/// Assert the registered editor commands are unique, publish shapes, and expire in time.
///
/// # Panics
///
/// Panics when a command's deferral budget would outlive the socket's wait for its reply.
pub fn assert_editor_command_set_is_conformant() {
    gdtf_qa_command::test_support::assert_unique_names(EDITOR_COMMANDS);
    gdtf_qa_command::test_support::assert_schemas_parse(EDITOR_COMMANDS);
    gdtf_qa_command::test_support::assert_shape_names_agree(EDITOR_COMMANDS);
    for command in EDITOR_COMMANDS {
        let budget = command.deferred_budget();
        assert!(
            *budget < *DEFAULT_REPLY_TIMEOUT,
            "`{}`'s deferral budget ({:?}) must expire strictly before the socket stops waiting \
             for the reply ({:?})",
            command.name().as_str(),
            *budget,
            *DEFAULT_REPLY_TIMEOUT
        );
    }
}

/// Names of all registered editor commands.
#[must_use]
pub fn editor_command_names() -> Vec<CommandName> {
    EDITOR_COMMANDS
        .iter()
        .map(|command| command.name())
        .collect()
}
