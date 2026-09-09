//! Hello and version negotiation over the real editor listener, in Editing and during Load.
use cobalt_mcp_protocol::message::{McpRequest, ProtocolVersion};

use crate::{
    mcp_hello::{
        assertions::{
            AnsweringPhase, assert_editor_catalogue, assert_unknown_command,
            assert_version_mismatch,
        },
        client::{exchange_while_editing, wrong_version},
    },
    mcp_shared::{
        harness::{advance_to_editing, editor_app_listening},
        hello::assert_hello_ok,
        load_case::reply_answered_during_load,
        support::TestResult,
    },
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
        McpRequest::Hello(ProtocolVersion::CURRENT),
        "the Hello(CURRENT) handshake",
    )?;
    assert_hello_ok(&reply);
    Ok(())
}

#[test]
fn a_wrong_version_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        McpRequest::Hello(wrong_version()),
        "the mismatched-version handshake",
    )?;
    assert_version_mismatch(&reply);
    Ok(())
}

#[test]
fn the_editors_own_drain_answers_while_it_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(McpRequest::Catalogue, "the catalogue request")?;
    assert_editor_catalogue(&reply, AnsweringPhase::Load);
    Ok(())
}
