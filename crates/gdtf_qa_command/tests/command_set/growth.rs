//! Adding a command moves no version.
//!
//! This is the whole point of the design, so it is a real test rather than a comment: the
//! same host, one more entry in its list, and everything a client sees grows by exactly one
//! row — while `ProtocolVersion::CURRENT` is untouched, because a command is DATA inside
//! two frozen envelope variants, not a variant of its own.

use gdtf_qa_command::{
    catalogue::catalogue,
    command::QaCommand,
    test_support::{
        FAKE_COMMANDS, FAKE_COMMANDS_GROWN, FakeEcho, FakeEchoReply, FakeEchoText, fake_app,
        fake_facts_loaded, fake_host_name, run_fake_command,
    },
};
use gdtf_qa_protocol::{command::CommandName, envelope::ProtocolVersion};

use crate::support::{args, plain, ran};

/// The catalogue grows by exactly one entry, and the new one is the added command.
#[test]
fn the_catalogue_grows_by_exactly_one_entry() {
    let facts = fake_facts_loaded();
    let before = catalogue(fake_host_name(), FAKE_COMMANDS, &facts);
    let after = catalogue(fake_host_name(), FAKE_COMMANDS_GROWN, &facts);

    assert_eq!(after.entries.len(), before.entries.len() + 1);
    assert_eq!(
        after.entries[..before.entries.len()],
        before.entries[..],
        "the existing rows are unchanged"
    );
    assert_eq!(after.entries[before.entries.len()].command, FakeEcho::NAME);
}

/// The added command resolves and runs, with no other edit anywhere.
#[test]
fn the_added_command_resolves_and_runs() {
    let mut app = fake_app(FAKE_COMMANDS_GROWN, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_GROWN,
        &FakeEcho::NAME,
        &args("{\"text\":\"hello\"}"),
        &plain(),
    );

    app.update();

    let reply: FakeEchoReply = ran(&channel);
    assert_eq!(
        reply,
        FakeEchoReply {
            text: FakeEchoText::new("hello".to_owned()),
        }
    );
}

/// The protocol version is unchanged by any of the above.
///
/// `14` is where the command-carrying envelope stands: GTW-939 landed it at `13`, and GTW-942
/// moved it once, for two FIELDS added to shipped shapes (`RunCommand.options` and
/// `CommandEntry.timing`) — not for a command. Adding fake commands #3 and #4 above changed no
/// request variant, no response variant and no field, so this number has no reason to move —
/// and a change to it would be a change to the very thing this design exists to stop touching.
#[test]
fn adding_a_command_moves_no_version() {
    let observed = *ProtocolVersion::CURRENT;

    // Everything the growth test does, again, in one place.
    let facts = fake_facts_loaded();
    let before = catalogue(fake_host_name(), FAKE_COMMANDS, &facts);
    let after = catalogue(fake_host_name(), FAKE_COMMANDS_GROWN, &facts);
    assert_eq!(after.entries.len(), before.entries.len() + 1);

    let mut app = fake_app(FAKE_COMMANDS_GROWN, facts);
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_GROWN,
        &FakeEcho::NAME,
        &args("{\"text\":\"still 14\"}"),
        &plain(),
    );
    app.update();
    let reply: FakeEchoReply = ran(&channel);
    assert_eq!(reply.text.as_str(), "still 14");

    assert_eq!(
        *ProtocolVersion::CURRENT,
        observed,
        "adding a command must not move the protocol version"
    );
    assert_eq!(
        *ProtocolVersion::CURRENT,
        14,
        "the command-carrying envelope stands at 14"
    );
}

/// The added command is reachable by NAME through the same resolution path as the rest —
/// there is no second registry it had to be added to.
#[test]
fn the_added_name_resolves_through_the_one_list() {
    let published = catalogue(fake_host_name(), FAKE_COMMANDS_GROWN, &fake_facts_loaded());
    assert!(
        published
            .entries
            .iter()
            .any(|entry| entry.command.as_str() == "fake.echo"),
        "the added command is published in the catalogue"
    );
    assert_eq!(
        FakeEcho::NAME,
        CommandName::from_static("fake.echo"),
        "the published name is the command's own const"
    );
}
