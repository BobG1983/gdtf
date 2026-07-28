//! GTW-805: the editor's ADR 0007 query pair over the EDITOR's REAL net-QA listener, driven
//! against the REAL editor app (re-homed in GTW-879).
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
//! so the doc survives a feature-off build — the GTW-804 `net_qa_hello` precedent) compiles
//! the whole dir-form suite to an empty crate without the feature. Run it with the feature
//! on: `cargo test -p gdtf_content_editor --features net_qa --test net_qa_editor_query`.
//!
//! Everything goes through the production path, on BOTH sides. The socket half: the editor's
//! own `NetQaEditorPlugin` binds a REAL loopback listener on an OS-assigned port, the client
//! is a REAL `TcpStream` speaking the REAL framing codec, and the replies come from the
//! plugin's REAL request drain. The editor half: the app is the REAL `MapEditorPlugin` on the
//! no-renderer `DefaultPlugins` UI harness (`harness.rs`), so every answer is read off the
//! model the editor's OWN `Load` pass and `OnEnter(Editing)` lifecycle produced. GTW-879
//! replaced a bare app that never ran the editor's plugin at all, with the authoring model
//! hand-inserted into its world: that could only ever pin a stand-in's behavior as the
//! contract — and did, asserting a `Load`-phase topic list and a `checks_complete` flag a
//! real editor contradicts.
//!
//! Two phases over ONE connection (the transport serves one client at a time):
//!
//! 1. During `Load` — while the editor is still resolving its content registries —
//!    `GetEditorQueryOptions` reports readiness `Load` and offers the topics whose resources
//!    already exist: `Readiness` and `Validation`. Both are then queried and both are
//!    ANSWERED, so the offer is real and not merely advertised; a topic that is NOT offered
//!    (`Mode`) is refused `BadRequest` rather than answered with an invented empty view. The
//!    client puts the first request on the wire BEFORE the test body runs a frame
//!    (`client::drive`), so that reply is a `Load` observation by construction rather than a
//!    race with the asset pass — which finishes in as few as six frames under parallel
//!    `cargo` contention.
//! 2. The app is then driven to `Editing` by the editor's own transition. The client POLLS
//!    `GetEditorQueryOptions` until it reports readiness `Editing` — the documented way an
//!    inject loop waits out the asset pass instead of racing it — and the options then list
//!    exactly `EditorQueryKind::ALL`. Each topic is queried and its view asserted against
//!    what the real editor opened with.
//!
//! ## Members (one concern per file)
//!
//! - [`support`] — the shared aliases and loop caps.
//! - [`harness`] — the real editor app + the drive-to-`Editing` driver.
//! - [`client`] — the socket half: the framed exchanges and the readiness poll.
//! - [`assertions`] — the per-phase assertions over the collected replies.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod assertions;
mod client;
mod harness;
mod support;

use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
};

use bevy::prelude::*;
use gdtf_qa_protocol::envelope::QaResponse;

use crate::{
    assertions::{assert_editing_phase, assert_load_phase},
    client::drive,
    harness::{advance_to_editing, editor_app_listening},
    support::{
        LOAD_PHASE_REPLIES, OPEN_WAIT, PHASE_UPDATES, POLL_STEP, PhaseReport, TestError, TestResult,
    },
};

/// The editor answers its ADR 0007 query pair over the real listener, off the real editor's
/// own model: readiness-filtered options during its `Load` asset pass, then every topic once
/// its `Editing` lifecycle is up.
#[test]
fn editor_query_pair_answers_over_the_real_editor_listener() -> TestResult {
    let (mut app, port) = editor_app_listening()?;

    let (opened_tx, opened_rx) = mpsc::channel();
    let (phase_tx, phase_rx) = mpsc::channel();
    thread::spawn(move || {
        if let Err(failure) = drive(port, &opened_tx, &phase_tx) {
            let _sent = phase_tx.send(Err(failure));
        }
    });

    // NOT ONE FRAME before the first request is on the wire. The accept loop is its own
    // thread, so this wait costs the app nothing — and it is what makes the `Load` phase an
    // observation instead of a race: the first reply is produced on one of the editor's first
    // frames, when `Startup` has only just asked for the content folders.
    opened_rx.recv_timeout(OPEN_WAIT)?;
    // A fixed-size array, not the collected `Vec`: `client::drive` builds exactly
    // `LOAD_PHASE_REPLIES` replies and `assert_load_phase` destructures exactly that many, so
    // the two can only drift apart by failing to compile. A short read is a client-half
    // failure and reports as one rather than as a panic inside the assertions.
    let load_phase: [QaResponse; LOAD_PHASE_REPLIES] = collect_phase(&mut app, &phase_rx)?
        .try_into()
        .map_err(|replies: Vec<QaResponse>| {
            TestError::from(format!(
                "expected exactly {LOAD_PHASE_REPLIES} Load-phase replies, got {replies:?}"
            ))
        })?;
    assert_load_phase(&load_phase);

    // Let the editor's own `Load → Editing` transition fire. The client is polling
    // `GetEditorQueryOptions` throughout; each poll is answered by one of these frames.
    advance_to_editing(&mut app);

    let editing_phase = collect_phase(&mut app, &phase_rx)?;
    assert_editing_phase(&editing_phase);
    Ok(())
}

/// Drive one frame per iteration until the client half reports its next phase.
///
/// The replies only exist once the plugin's drain has run in `Update`, so the app must be
/// ticked while the client waits — and each iteration waits [`POLL_STEP`] for the report
/// before ticking again, which paces the drive to the client rather than to a frame budget.
fn collect_phase(app: &mut App, rx: &Receiver<PhaseReport>) -> Result<Vec<QaResponse>, TestError> {
    for _ in 0..PHASE_UPDATES {
        app.update();
        match rx.recv_timeout(POLL_STEP) {
            Ok(report) => return report,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Err("the client half never reported a phase".into())
}
