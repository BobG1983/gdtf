//! Adding a command moves no version.
//! row — while `ProtocolVersion::CURRENT` is untouched, because a command is DATA inside
use gdtf_qa_command::{
    catalogue::catalogue,
    command::QaCommand,
    test_support::{
        FAKE_COMMANDS, FAKE_COMMANDS_GROWN, FakeEcho, FakeEchoReply, FakeEchoText, fake_app,
        fake_facts_loaded, fake_host_name, run_fake_command,
    },
};
use gdtf_qa_protocol::{command::CommandName, message::ProtocolVersion};

use crate::support::{args, plain, ran};

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

#[test]
fn the_added_command_resolves_and_runs() {
    let mut app = fake_app(FAKE_COMMANDS_GROWN, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_GROWN,
        &FakeEcho::NAME,
        &args("(text:\"hello\")"),
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

#[test]
fn adding_a_command_moves_no_version() {
    let observed = *ProtocolVersion::CURRENT;

    let facts = fake_facts_loaded();
    let before = catalogue(fake_host_name(), FAKE_COMMANDS, &facts);
    let after = catalogue(fake_host_name(), FAKE_COMMANDS_GROWN, &facts);
    assert_eq!(after.entries.len(), before.entries.len() + 1);

    let mut app = fake_app(FAKE_COMMANDS_GROWN, facts);
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_GROWN,
        &FakeEcho::NAME,
        &args("(text:\"still 15\")"),
        &plain(),
    );
    app.update();
    let reply: FakeEchoReply = ran(&channel);
    assert_eq!(reply.text.as_str(), "still 15");

    assert_eq!(
        *ProtocolVersion::CURRENT,
        observed,
        "adding a command must not move the protocol version"
    );
    assert_eq!(
        *ProtocolVersion::CURRENT,
        15,
        "the command-carrying envelope stands at 15"
    );
}

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
