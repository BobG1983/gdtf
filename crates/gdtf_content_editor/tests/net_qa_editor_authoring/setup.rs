use bevy::app::App;

use crate::{
    draft_reply::{DraftOutcomeRow, DraftReplyRow},
    mirror::ModeRow,
    names::{
        EDITOR_DRAFT, EDITOR_LAST_SAVE, EDITOR_LIST_OP, EDITOR_NEW, EDITOR_SAVE, EDITOR_SET_FIELD,
        EDITOR_SET_MODE,
    },
    outcome::{ran_body, unavailable_code},
    rows::{
        LastSaveReplyRow, LastSaveRow, ListOpReplyRow, NewReplyRow, SaveOutcomeRow, SaveReplyRow,
        SetFieldReplyRow, SetModeReplyRow,
    },
    socket::{Client, run_editor},
    support::TestError,
};

/// Run `editor.set_mode` and read the tab the reply says is open.
pub(crate) fn set_mode(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<SetModeReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SET_MODE, arguments))?;
    ran_body(&reply, EDITOR_SET_MODE)
}

/// Run `editor.new` and read what it did to the draft.
pub(crate) fn new_draft(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<NewReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_NEW, arguments))?;
    ran_body(&reply, EDITOR_NEW)
}

/// Run `editor.set_field` and read the field the reply carries.
pub(crate) fn set_field(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<SetFieldReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SET_FIELD, arguments))?;
    ran_body(&reply, EDITOR_SET_FIELD)
}

/// Run `editor.list_op` and read the list the reply carries.
pub(crate) fn list_op(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<ListOpReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_LIST_OP, arguments))?;
    ran_body(&reply, EDITOR_LIST_OP)
}

/// Run `editor.save` and read what the writer reported.
pub(crate) fn save(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<SaveOutcomeRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SAVE, arguments))?;
    let body: SaveReplyRow = ran_body(&reply, EDITOR_SAVE)?;
    Ok(body.outcome)
}

/// Run `editor.draft` and read the tab it answered for and the RON its draft projects to.
pub(crate) fn draft_ron(
    app: &mut App,
    client: &mut Client,
) -> Result<(ModeRow, String), TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_DRAFT, "()"))?;
    let body: DraftReplyRow = ran_body(&reply, EDITOR_DRAFT)?;
    let DraftOutcomeRow::Ron(text) = body.outcome else {
        return Err(format!("the draft must project to RON, got {:?}", body.outcome).into());
    };
    Ok((body.mode, text))
}

/// Run `editor.draft` before a form tab is open and read the refusal code it answers.
pub(crate) fn draft_refusal(app: &mut App, client: &mut Client) -> Result<String, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_DRAFT, "()"))?;
    unavailable_code(&reply)
}

/// Run `editor.last_save` for one mode and read the records the reply carries.
pub(crate) fn last_save_rows(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<Vec<LastSaveRow>, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_LAST_SAVE, arguments))?;
    let body: LastSaveReplyRow = ran_body(&reply, EDITOR_LAST_SAVE)?;
    Ok(body.records)
}
