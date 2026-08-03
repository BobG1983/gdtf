use gdtf_qa_protocol::command::CommandName;

use super::set::GAME_COMMANDS;

pub fn assert_game_command_set_is_conformant() {
    gdtf_qa_command::test_support::assert_unique_names(GAME_COMMANDS);
    gdtf_qa_command::test_support::assert_schemas_parse(GAME_COMMANDS);
}

#[must_use]
pub fn game_command_names() -> Vec<CommandName> {
    GAME_COMMANDS.iter().map(|command| command.name()).collect()
}
