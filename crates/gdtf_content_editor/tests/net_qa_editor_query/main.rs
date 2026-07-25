//! GTW-805: the editor's ADR 0007 query pair over the EDITOR's REAL net-QA listener.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
//! so the doc survives a feature-off build — the GTW-804 `net_qa_hello` precedent) compiles
//! the whole dir-form suite to an empty crate without the feature. Run it with the feature
//! on: `cargo test -p gdtf_content_editor --features net_qa --test net_qa_editor_query`.
//!
//! Everything goes through the production path: the editor's own `NetQaEditorPlugin` binds a
//! REAL loopback listener on an OS-assigned port, the client is a REAL `TcpStream` speaking
//! the REAL framing codec, and the replies come from the plugin's REAL request drain running
//! in its `Gather` band under the editor's own `EditorState` machine.
//!
//! Two phases over ONE connection (the transport serves one client at a time):
//!
//! 1. During `Load` — before any authoring model exists — `GetEditorQueryOptions` reports
//!    readiness `Load` and offers ONLY the `Readiness` topic; `QueryEditor(Readiness)` is
//!    answered and carries readiness `Load`; a model-backed topic is refused `BadRequest`
//!    rather than answered with an invented empty view.
//! 2. The test body then inserts the state-scoped model and enters `Editing`. The client
//!    POLLS `GetEditorQueryOptions` until it reports readiness `Editing` — the documented way
//!    an inject loop waits out the asset pass instead of racing it — and the options then
//!    list exactly `EditorQueryKind::ALL`. Each topic is queried and its view asserted.
//!
//! ## Members (one concern per file)
//!
//! - [`support`] — the shared aliases, loop caps and fixture constants.
//! - [`client`] — the socket half: the framed exchanges and the readiness poll.
//! - [`assertions`] — the per-phase assertions over the collected replies.
//! - [`fixture`] — the authoring model the `Editing` phase reads.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod assertions;
mod client;
mod fixture;
mod support;

use std::{
    sync::mpsc::{self, RecvTimeoutError},
    thread,
};

use bevy::{MinimalPlugins, prelude::*, state::app::StatesPlugin};
use gdtf_content_editor::{EditorState, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::envelope::QaResponse;

use crate::{
    assertions::{assert_editing_phase, assert_load_phase},
    client::drive,
    fixture::open_the_editing_scene,
    support::{MAX_UPDATES, POLL_STEP, TestResult},
};

/// The editor answers its ADR 0007 query pair over the real listener: readiness-filtered
/// options during `Load`, then every topic once `Editing` is up.
#[test]
fn editor_query_pair_answers_over_the_real_editor_listener() -> TestResult {
    // Bind the REAL listener up front on an OS-assigned port, so the client knows where to
    // connect without racing the app's first `build`.
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        // The editor's OWN state machine — the run condition the drain is registered under.
        .add_plugins(StatesPlugin)
        .init_state::<EditorState>()
        .add_plugins(plugin);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        if let Err(failure) = drive(port, &tx) {
            let _sent = tx.send(Err(failure));
        }
    });

    // Drive real frames; the phases only complete once the plugin's drain has run in
    // `Update`. The model is inserted (and `Editing` entered) only AFTER the Load-phase
    // report arrives, so the Load-phase observations are not racing the transition.
    let mut phases: Vec<Vec<QaResponse>> = Vec::new();
    for _ in 0..MAX_UPDATES {
        app.update();
        match rx.recv_timeout(POLL_STEP) {
            Ok(report) => {
                phases.push(report?);
                if phases.len() == 1 {
                    open_the_editing_scene(&mut app);
                } else {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let [load_phase, editing_phase] = phases.as_slice() else {
        return Err(format!("expected both phases to report, got {phases:?}").into());
    };
    assert_load_phase(load_phase);
    assert_editing_phase(editing_phase);
    Ok(())
}
