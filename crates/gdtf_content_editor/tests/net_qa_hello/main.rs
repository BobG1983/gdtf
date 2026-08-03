//! GTW-804: Hello/version negotiation over the EDITOR's REAL net-QA listener, on BOTH sides of
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod assertions;
mod client;
mod drive;
mod harness;
mod load_case;
mod support;

use std::{sync::mpsc, thread};

use gdtf_qa_protocol::message::{ProtocolVersion, QaRequest};

use crate::{
    assertions::{
        assert_editor_catalogue, assert_hello_ok, assert_unknown_command, assert_version_mismatch,
    },
    client::{exchange_while_editing, wrong_version},
    drive::drive_until_batched,
    harness::{advance_to_editing, editor_app_listening},
    load_case::reply_answered_during_load,
    support::TestResult,
};

#[test]
fn hello_negotiates_over_the_real_editor_listener() -> TestResult {
    let (mut app, port) = editor_app_listening()?;
    advance_to_editing(&mut app);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _sent = tx.send(exchange_while_editing(port));
    });

    let [hello, mismatch, unknown] = drive_until_batched(&mut app, &rx)?;
    assert_hello_ok(&hello);
    assert_version_mismatch(&mismatch);
    assert_unknown_command(&unknown);
    Ok(())
}

#[test]
fn hello_negotiates_while_the_editor_is_still_loading() -> TestResult {
    let (reply, _) = reply_answered_during_load(
        QaRequest::Hello(ProtocolVersion::CURRENT),
        "the Hello(CURRENT) handshake",
    )?;
    assert_hello_ok(&reply);
    Ok(())
}

#[test]
fn a_wrong_version_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let (reply, _) = reply_answered_during_load(
        QaRequest::Hello(wrong_version()),
        "the mismatched-version handshake",
    )?;
    assert_version_mismatch(&reply);
    Ok(())
}

#[test]
fn the_editors_own_drain_answers_while_it_is_still_loading() -> TestResult {
    let (reply, during) =
        reply_answered_during_load(QaRequest::Catalogue, "the catalogue request")?;
    assert_editor_catalogue(&reply, during.as_ref());
    Ok(())
}
