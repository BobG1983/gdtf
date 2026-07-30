//! The shared body of the `Load`-phase cases (GTW-896): issue ONE request while the editor is
//! still in [`EditorState::Load`] and hand back the reply it was answered with.
//!
//! Nothing is hand-inserted and no state is forced. The app is in `Load` because that is
//! [`EditorState::default`] under [`MapEditorPlugin`](gdtf_content_editor::MapEditorPlugin)'s own
//! lifecycle (GTW-879's clause 2), and it is still there when the reply lands because the
//! request was already pending before frame 1 while the editor's own gate needs seven frames to
//! resolve its registries.

use std::{sync::mpsc, thread};

use gdtf_content_editor::EditorState;
use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};

use crate::{
    assertions::assert_answered_during_load,
    client::exchange_during_load,
    drive::drive_until_tagged,
    harness::{editor_app_listening, editor_state},
    support::{OPEN_WAIT, TestError},
};

/// Answer `request` with the reply the editor produced for it during its `Load` asset pass.
///
/// `what` names the request in the failure messages, so a case that slipped past the transition
/// says which one it was.
///
/// # Errors
///
/// Any listener-bind, socket, codec or channel failure, or a frame budget spent before the reply
/// arrived.
pub(crate) fn reply_answered_during_load(
    request: QaRequest,
    what: &str,
) -> Result<(QaResponse, Option<EditorState>), TestError> {
    // Bind the REAL listener up front on an OS-assigned port, so the client knows where to
    // connect without racing the app's first `build`.
    let (mut app, port) = editor_app_listening()?;

    let (opened_tx, opened_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();
    thread::spawn(move || {
        let _sent = reply_tx.send(exchange_during_load(port, &opened_tx, request));
    });

    // NOT ONE FRAME before the request is on the wire. The accept loop is its own thread, so
    // this wait costs the app nothing — and it is what makes the `Load` observation an
    // observation instead of a race: the reply is produced on one of the editor's first frames,
    // when `Startup` has only just asked for the content folders.
    opened_rx.recv_timeout(OPEN_WAIT)?;
    assert_eq!(
        editor_state(&app),
        Some(EditorState::Load),
        "the app has run no frames yet, so the editor's own state machine must still be in its \
         default Load pass when {what} goes out",
    );

    let (reply, during) = drive_until_tagged(&mut app, &reply_rx)?;
    assert_answered_during_load(during.as_ref(), what);
    Ok((reply, during))
}
