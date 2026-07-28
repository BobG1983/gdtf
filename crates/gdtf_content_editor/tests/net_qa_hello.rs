//! GTW-804: Hello/version negotiation over the EDITOR's REAL net-QA listener.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
//! so the doc survives a feature-off build — the `gdtf_app` `net_qa` suite precedent) compiles
//! the file to an empty crate without the feature: the `net_qa` module it exercises does not
//! exist there.
//!
//! Everything here goes through the production path, on BOTH sides. The socket half: the
//! editor's own [`NetQaEditorPlugin`] binds a REAL loopback listener (on an OS-assigned
//! ephemeral port, so parallel test binaries never collide) and spawns the shared transport's
//! REAL accept loop; the client is a REAL [`TcpStream`] speaking the REAL framing codec; and
//! the replies are produced by the plugin's REAL request drain, running in its
//! [`EditorNetQaSystems::Gather`](gdtf_content_editor::EditorNetQaSystems) band under the
//! editor's own [`EditorState`] machine — no stub, no injected channel, no hand-called system.
//! The editor half (re-homed in GTW-879): the app is the REAL [`MapEditorPlugin`] on the
//! no-renderer `DefaultPlugins` UI harness — the recipe `tests/prefab_mode/harness.rs` uses
//! and 13 other editor test files already follow — driven to [`EditorState::Editing`] by the
//! editor's own `Load` asset pass before a single request goes out. The bare app this
//! replaced never ran the editor's plugin at all, which made "the real listener path" true of
//! the socket and false of the editor.
//!
//! Three exchanges over ONE connection (the transport serves one client at a time, so a single
//! connection also avoids racing the slot's release):
//!
//! 1. `Hello(CURRENT)` negotiates — `HelloOk` carrying the editor's own server identity;
//! 2. `Hello(CURRENT + 1)` is rejected `VersionMismatch` — proving real negotiation rather
//!    than a canned reply;
//! 3. `GetAppFlow` is rejected `BadRequest` — the editor services no other request in this
//!    child (those are GTW-805 / GTW-806 / GTW-808).
#![cfg(all(debug_assertions, feature = "net_qa"))]

use std::{
    error::Error,
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use bevy::prelude::*;
use gdtf_content_editor::{EDITOR_QA_SERVER_NAME, EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaError, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A boxed error so a test's `?` spans `io::Error`, the codec's error type and a bare
/// message. `Send + Sync` because the client half's result crosses a thread boundary back to
/// the test body over an [`mpsc`] channel.
type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
type TestResult = Result<(), TestError>;

/// The client thread's result — the replies it collected, in order.
type ClientResult = Result<Vec<QaResponse>, TestError>;

/// A SAFETY NET on the frame loop, not a timing budget: the loop exits as soon as the client
/// thread reports, and each iteration waits up to [`POLL_STEP`] for it.
const MAX_UPDATES: u32 = 400;

/// A SAFETY NET on the frames the editor's own `Load` asset pass may take — the same generous
/// cap `tests/prefab_mode/harness.rs` uses, because an async folder load under parallel
/// `cargo` contention takes a non-deterministic number of frames.
const EDITING_UPDATES: u32 = 10_000;

/// How long one frame iteration waits on the client thread before driving another update.
const POLL_STEP: Duration = Duration::from_millis(10);

/// Build the real editor app with its QA listener already bound, returning the app and the
/// port the OS assigned.
///
/// The listener is pre-bound through [`NetQaEditorPlugin::listening`] because the env gate the
/// binary uses cannot be driven from a test (`std::env::set_var` is `unsafe` in edition 2024
/// and the workspace forbids `unsafe`); everything downstream of the bind is the production
/// path. [`MapEditorPlugin`] is the SAME plugin the binary wires, minus the windowed
/// [`EguiPlugin`](bevy_egui::EguiPlugin) the headless harness has no window for.
fn editor_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    Ok((app, port))
}

/// Drive the app until the editor's own `Load` pass releases into [`EditorState::Editing`],
/// then a few frames so the `OnEnter(Editing)` inserts apply.
fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|state| *state.get() == EditorState::Editing)
        },
        EDITING_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         registries (a genuine load failure, not a frame-budget shortfall)",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The client half: connect to the editor's real listener and run the three exchanges over
/// ONE connection, returning the replies in order.
fn exchange(port: NetQaPort) -> ClientResult {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    // Bound the client's own read so a failure surfaces as an error rather than a hang.
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    // ONE decoder across the whole connection: a reply may arrive in the same read as the
    // tail of the previous one, and the decoder owns that leftover.
    let mut decoder = FrameDecoder::new();
    let mut replies = Vec::new();
    let requests = [
        QaRequest::Hello(ProtocolVersion::CURRENT),
        QaRequest::Hello(ProtocolVersion::new(*ProtocolVersion::CURRENT + 1)),
        QaRequest::GetAppFlow,
    ];
    for request in requests {
        stream.write_all(&encode(&request)?)?;
        replies.push(read_response(&mut stream, &mut decoder)?);
    }
    Ok(replies)
}

/// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
fn read_response(
    stream: &mut TcpStream,
    decoder: &mut FrameDecoder,
) -> Result<QaResponse, TestError> {
    let mut buf = [0u8; 512];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<QaResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        assert!(read > 0, "the editor closed before a full response arrived");
        decoder.push(&buf[..read]);
    }
}

/// Hello negotiates over the editor's real listener, a mismatched version is refused, and no
/// other request kind is serviced yet.
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
        let _sent = tx.send(exchange(port));
    });

    // Drive real frames until the client thread reports: the replies only exist once the
    // plugin's drain has run in `Update`.
    let mut reported = None;
    for _ in 0..MAX_UPDATES {
        app.update();
        match rx.recv_timeout(POLL_STEP) {
            Ok(result) => {
                reported = Some(result);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let replies = reported.ok_or("the client thread never reported an exchange")??;
    let [hello, mismatch, unsupported] = replies.as_slice() else {
        return Err(format!("expected exactly three replies, got {replies:?}").into());
    };

    assert!(
        matches!(
            hello,
            QaResponse::HelloOk(facts)
                if facts.protocol == ProtocolVersion::CURRENT
                    && *facts.server == EDITOR_QA_SERVER_NAME
        ),
        "expected the editor's HelloOk handshake facts, got {hello:?}",
    );
    assert!(
        matches!(mismatch, QaResponse::Error(QaError::VersionMismatch)),
        "expected VersionMismatch for a wrong client version, got {mismatch:?}",
    );
    assert!(
        matches!(unsupported, QaResponse::Error(QaError::BadRequest)),
        "expected BadRequest for a request the editor does not service yet, got {unsupported:?}",
    );
    Ok(())
}
