use std::{sync::mpsc, thread};

use gdtf_content_editor::EditorState;
use gdtf_qa_protocol::message::{QaRequest, QaResponse};

use crate::{
    assertions::assert_answered_during_load,
    client::exchange_during_load,
    drive::drive_until_tagged,
    harness::{editor_app_listening, editor_state},
    support::{OPEN_WAIT, TestError},
};

pub(crate) fn reply_answered_during_load(
    request: QaRequest,
    what: &str,
) -> Result<(QaResponse, Option<EditorState>), TestError> {
    let (mut app, port) = editor_app_listening()?;

    let (opened_tx, opened_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();
    thread::spawn(move || {
        let _sent = reply_tx.send(exchange_during_load(port, &opened_tx, request));
    });

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
