//! The inbox takes only its own command's calls, and the decode step's two outcomes.

use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName, CommandOutcome},
    message::QaResponse,
};

use crate::{
    command::QaCommand,
    dispatch::{CommandInbox, bad_arguments},
    test_support::{FakeCell, FakePhase},
};

/// `take_for` takes every call for one name and leaves the others in order.
#[test]
fn the_inbox_takes_only_the_named_command_s_calls() {
    let mut inbox = CommandInbox::default();
    for name in [FakePhase::NAME, FakeCell::NAME, FakePhase::NAME] {
        let (responder, _answer) = Responder::channel();
        inbox.admit(name, CommandArgsJson::new("{}".to_owned()), responder);
    }
    assert_eq!(inbox.len(), 3);

    let taken = inbox.take_for(&FakePhase::NAME);
    assert_eq!(taken.len(), 2, "both fake.phase calls come out");
    assert_eq!(inbox.len(), 1, "the fake.cell call stays");
    assert!(!inbox.is_empty());

    let rest = inbox.take_for(&FakeCell::NAME);
    assert_eq!(rest.len(), 1);
    assert!(inbox.is_empty());
}

/// A name nothing was admitted under takes nothing and leaves the inbox untouched.
#[test]
fn the_inbox_takes_nothing_for_an_unused_name() {
    let mut inbox = CommandInbox::default();
    let (responder, _answer) = Responder::channel();
    inbox.admit(
        FakePhase::NAME,
        CommandArgsJson::new("{}".to_owned()),
        responder,
    );

    let taken = inbox.take_for(&CommandName::from_static("fake.nothing"));
    assert!(taken.is_empty());
    assert_eq!(inbox.len(), 1);
}

/// `bad_arguments::<C>` attaches C's OWN derived argument schema, not a generic one.
#[test]
fn bad_arguments_attaches_the_command_s_own_schema() {
    let Err(fault) = serde_json::from_str::<<FakeCell as QaCommand>::Args>("{\"nope\":1}") else {
        unreachable!("`{{\"nope\":1}}` must not decode into FakeCellArgs");
    };
    let response = bad_arguments::<FakeCell>(&fault);

    let QaResponse::Outcome(CommandOutcome::BadArguments { detail, schema }) = response else {
        unreachable!("a decode failure must answer BadArguments, got {response:?}");
    };
    assert!(
        !detail.as_str().is_empty(),
        "the decoder's own words ride along"
    );
    assert!(
        schema.as_str().contains("\"cell\""),
        "the attached schema is FakeCellArgs' own: {}",
        schema.as_str()
    );
}
