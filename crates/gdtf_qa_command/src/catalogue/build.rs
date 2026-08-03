use gdtf_qa_protocol::{
    command::{CommandCatalogue, CommandEntry},
    message::ServerNameNet,
};

use crate::command::ErasedCommand;

#[must_use]
pub fn catalogue<F>(
    host: ServerNameNet,
    commands: &[&dyn ErasedCommand<F>],
    facts: &F,
) -> CommandCatalogue {
    let entries = commands
        .iter()
        .map(|command| {
            CommandEntry::new(
                command.name(),
                command.summary(),
                command.timing(),
                command.arg_schema(),
                command.reply_schema(),
                command.availability(facts),
            )
        })
        .collect();
    CommandCatalogue::new(host, entries)
}
