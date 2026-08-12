use gdtf_content_editor::EditorState;
use gdtf_qa_protocol::message::{ProtocolVersion, QaRequest, QaResponse};

use crate::{
    assertions::{assert_answered_during_load, assert_hello_ok},
    client::Client,
    harness::{editor_app_listening, editor_state},
    support::TestError,
};

pub(crate) fn reply_answered_during_load(
    request: QaRequest,
    what: &str,
) -> Result<(QaResponse, Option<EditorState>), TestError> {
    let (mut app, port) = editor_app_listening()?;
    let mut client = Client::connect(port)?;
    client.send(&QaRequest::Hello(ProtocolVersion::CURRENT))?;
    client.send(&request)?;
    assert_eq!(
        editor_state(&app),
        Some(EditorState::Load),
        "the app has run no frames yet, so the editor's own state machine must still be in its \
         default Load pass when {what} goes out",
    );

    let negotiated = client.read(&mut app)?;
    assert_hello_ok(&negotiated);
    let reply = client.read(&mut app)?;
    let during = editor_state(&app);
    assert_answered_during_load(during.as_ref(), what);
    Ok((reply, during))
}
