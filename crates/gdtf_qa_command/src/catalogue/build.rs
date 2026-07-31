//! [`catalogue`] — the host's published command list, read from the frame's facts.

use gdtf_qa_protocol::{
    command::{CommandCatalogue, CommandEntry},
    envelope::ServerNameNet,
};

use crate::command::ErasedCommand;

/// Build the catalogue a host answers `Catalogue` with.
///
/// Every row's two schemas are DERIVED from the command's own argument and reply types, and
/// its availability comes from the command's own predicate — the same call
/// [`admit()`](crate::dispatch::admit()) makes on a `Run`. Advertised and admitted are
/// therefore computed from one call per command and cannot disagree, which is the property
/// `tests/command_set/one_predicate.rs` pins.
///
/// The rows come out in the slice's order, so a host controls catalogue order by ordering
/// its list.
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
                command.arg_schema(),
                command.reply_schema(),
                command.availability(facts),
            )
        })
        .collect();
    CommandCatalogue::new(host, entries)
}
