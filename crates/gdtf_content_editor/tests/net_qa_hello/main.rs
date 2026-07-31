//! GTW-804: Hello/version negotiation over the EDITOR's REAL net-QA listener, on BOTH sides of
//! the editor's `Load` → `Editing` transition (the `Load` side added in GTW-896).
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
//! so the doc survives a feature-off build — the `gdtf_app` `net_qa` suite precedent) compiles
//! the whole dir-form suite to an empty crate without the feature: the `net_qa` module it
//! exercises does not exist there. Run it with the feature on:
//! `cargo test -p gdtf_content_editor --features net_qa --test net_qa_hello`.
//!
//! Everything here goes through the production path, on BOTH sides. The socket half: the
//! editor's own [`NetQaEditorPlugin`](gdtf_content_editor::NetQaEditorPlugin) binds a REAL
//! loopback listener (on an OS-assigned ephemeral port, so parallel test binaries never
//! collide) and spawns the shared transport's REAL accept loop; the client is a REAL
//! [`TcpStream`](std::net::TcpStream) speaking the REAL framing codec; and the replies are
//! produced by the plugin's REAL request drain, running in its
//! [`EditorNetQaSystems::Gather`](gdtf_content_editor::EditorNetQaSystems) band under the
//! editor's own [`EditorState`](gdtf_content_editor::EditorState) machine — no stub, no injected
//! channel, no hand-called system.
//! The editor half (re-homed in GTW-879): the app is the REAL
//! [`MapEditorPlugin`](gdtf_content_editor::MapEditorPlugin) on the no-renderer
//! `DefaultPlugins` UI harness — the recipe `tests/prefab_mode/harness.rs` uses and 13 other
//! editor test files already follow. The bare app this replaced never ran the editor's plugin
//! at all, which made "the real listener path" true of the socket and false of the editor.
//!
//! ## The cases
//!
//! [`hello_negotiates_over_the_real_editor_listener`] drives the editor to
//! [`EditorState::Editing`](gdtf_content_editor::EditorState::Editing) through its own asset pass
//! first, then runs three exchanges over ONE connection: `Hello(CURRENT)` negotiates,
//! `Hello(CURRENT + 1)` is refused `VersionMismatch`, and `GetAppFlow` is refused `BadRequest`
//! (the editor runs no battle).
//!
//! Since GTW-940 the first two of those are answered in the LISTENER THREAD, from the facts
//! [`NetQaEditorPlugin`](gdtf_content_editor::NetQaEditorPlugin) hands
//! [`run_listener`](gdtf_net_qa_transport::run_listener) — so what they pin here is that the
//! editor hands over ITS OWN facts (the reply names `gdtf-editor-net-qa`, never the game).
//! The third still exercises the editor's drain, and now also proves the connection genuinely
//! negotiated: a `GetAppFlow` on a connection that had not would come back `NotNegotiated`
//! from the transport instead of `BadRequest` from the editor.
//!
//! The three GTW-896 cases put ONE request each on the wire while the editor is still in
//! [`EditorState::Load`](gdtf_content_editor::EditorState::Load) — the state an agent's
//! poll-until-ready loop actually handshakes in.
//! Before GTW-896 both editor QA suites reached `Editing` before their first request, so a
//! handshake that refused or altered itself before `Editing` would have passed every test.
//!
//! Their `Load` claim is a per-reply FACT rather than a timing bet, and the margin is measured
//! rather than hoped for:
//!
//! - The request is on the wire before the test body runs a single frame (`load_case.rs`), and
//!   the editor's own `Load` → `Editing` gate takes SEVEN frames in this harness, so the
//!   answering frame is deep inside the asset pass.
//! - ONE request per connection, one connection per case: the transport is lockstep per client,
//!   so every extra exchange would spend another frame of that margin. The handshake each
//!   connection now opens with (GTW-940) is not such an exchange — the listener thread answers
//!   it without the drain running, so it spends no frame.
//! - The reply is paired with the editor state of the frame that answered it (`drive.rs`), and
//!   [`hello_load_readiness_matches_the_editor_state`] cross-checks that pairing against the
//!   readiness the editor itself puts on the wire.
//!
//! ## Members (one concern per file)
//!
//! - [`support`] — the shared aliases and loop caps.
//! - [`harness`] — the real editor app, its state read, and the drive-to-`Editing` driver.
//! - [`client`] — the socket half: the framed exchanges for each side.
//! - [`drive`] — driving the app's frames while the client talks, tagging a reply with state.
//! - [`load_case`] — the shared body of the `Load`-phase cases.
//! - [`assertions`] — the assertions over the collected replies.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod assertions;
mod client;
mod drive;
mod harness;
mod load_case;
mod support;

use std::{sync::mpsc, thread};

use gdtf_qa_protocol::envelope::{ProtocolVersion, QaRequest};

use crate::{
    assertions::{
        assert_hello_ok, assert_readiness_matches_state, assert_unsupported,
        assert_version_mismatch,
    },
    client::{exchange_while_editing, wrong_version},
    drive::drive_until_batched,
    harness::{advance_to_editing, editor_app_listening},
    load_case::reply_answered_during_load,
    support::TestResult,
};

/// Hello negotiates over the editor's real listener once the editor is fully open, a mismatched
/// version is refused, and no other request kind is serviced yet.
#[test]
fn hello_negotiates_over_the_real_editor_listener() -> TestResult {
    // Bind the REAL listener up front on an OS-assigned port, so the client knows where to
    // connect without racing the app's first `build`.
    let (mut app, port) = editor_app_listening()?;
    // Open the editor for real before a request goes out: the drain runs under the editor's
    // OWN state machine (never the game's `AppState`, which this crate has no access to), and
    // the handshake this test pins must hold once the editor is fully up, not only while it is
    // still an empty shell.
    advance_to_editing(&mut app);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _sent = tx.send(exchange_while_editing(port));
    });

    let [hello, mismatch, unsupported] = drive_until_batched(&mut app, &rx)?;
    assert_hello_ok(&hello);
    assert_version_mismatch(&mismatch);
    assert_unsupported(&unsupported);
    Ok(())
}

/// GTW-896: the handshake answers the SAME facts while the editor is still in
/// [`EditorState::Load`](gdtf_content_editor::EditorState::Load) as it does once open.
///
/// The claim the ticket exists for: a handler that branched on `EditorState` — refusing or
/// altering the handshake during the asset pass — fails HERE, and passed everything before this
/// case existed. GTW-940 makes that structurally impossible rather than merely observed: the
/// handshake is answered in the listener thread and cannot read the editor's state at all.
#[test]
fn hello_negotiates_while_the_editor_is_still_loading() -> TestResult {
    let (reply, _) = reply_answered_during_load(
        QaRequest::Hello(ProtocolVersion::CURRENT),
        "the Hello(CURRENT) handshake",
    )?;
    assert_hello_ok(&reply);
    Ok(())
}

/// GTW-896: negotiation still NEGOTIATES before
/// [`EditorState::Editing`](gdtf_content_editor::EditorState::Editing) — a wrong client version
/// is refused for being a wrong version, not for arriving while the editor loads.
#[test]
fn a_wrong_version_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let (reply, _) = reply_answered_during_load(
        QaRequest::Hello(wrong_version()),
        "the mismatched-version handshake",
    )?;
    assert_version_mismatch(&reply);
    Ok(())
}

/// GTW-896: the wire's own account of the same window — the readiness the editor reports to a
/// polling client is the state its own machine was in on the answering frame.
///
/// This is what makes the two cases above readings of the editor rather than of the test's
/// bookkeeping: the state they tag their reply with is the state the editor itself publishes.
#[test]
fn hello_load_readiness_matches_the_editor_state() -> TestResult {
    let (reply, during) = reply_answered_during_load(
        QaRequest::GetEditorQueryOptions,
        "the readiness options request",
    )?;
    assert_readiness_matches_state(&reply, during.as_ref());
    Ok(())
}
