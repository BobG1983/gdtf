//! Hello/version negotiation, and the editor's lifecycle command layer — phase, mode tab, blank,
//! load, save and last-save — over the editor net-QA listener.
#![cfg(debug_assertions)]

mod assertions;
mod client;
mod command_set;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
mod last_save_command;
mod lifecycle;
mod list_op_command;
#[path = "../net_qa_shared/load_case.rs"]
mod load_case;
mod load_command;
#[path = "../net_qa_shared/mirror.rs"]
mod mirror;
mod new_command;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod phase_command;
mod phase_rows;
mod rows;
mod save_command;
mod set_field_command;
mod set_mode_command;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
#[path = "../net_qa_shared/support.rs"]
mod support;

use gdtf_qa_protocol::message::{ProtocolVersion, QaRequest};

use crate::{
    assertions::{
        AnsweringPhase, assert_editor_catalogue, assert_unknown_command, assert_version_mismatch,
    },
    client::{exchange_while_editing, wrong_version},
    harness::{advance_to_editing, editor_app_listening},
    hello::assert_hello_ok,
    load_case::reply_answered_during_load,
    support::TestResult,
};

#[test]
fn hello_negotiates_over_the_real_editor_listener() -> TestResult {
    let (mut app, port) = editor_app_listening()?;
    advance_to_editing(&mut app);

    let [hello, mismatch, unknown] = exchange_while_editing(&mut app, port)?;
    assert_hello_ok(&hello);
    assert_version_mismatch(&mismatch);
    assert_unknown_command(&unknown);
    Ok(())
}

#[test]
fn hello_negotiates_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        QaRequest::Hello(ProtocolVersion::CURRENT),
        "the Hello(CURRENT) handshake",
    )?;
    assert_hello_ok(&reply);
    Ok(())
}

#[test]
fn a_wrong_version_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        QaRequest::Hello(wrong_version()),
        "the mismatched-version handshake",
    )?;
    assert_version_mismatch(&reply);
    Ok(())
}

#[test]
fn the_editors_own_drain_answers_while_it_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(QaRequest::Catalogue, "the catalogue request")?;
    assert_editor_catalogue(&reply, AnsweringPhase::Load);
    Ok(())
}
