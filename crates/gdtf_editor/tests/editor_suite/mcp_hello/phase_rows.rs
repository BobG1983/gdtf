use cobalt_mcp_protocol::{command::CommandOutcome, message::McpResponse};
use gdtf_editor::{EditorMode, EditorState};
use serde::Deserialize;

use crate::mcp_shared::{mirror::ModeRow, support::TestError};

/// A client's own reading of the editor's lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum PhaseRow {
    Load,
    Editing,
}

/// `editor.phase`'s reply body as a client decodes it.
#[derive(Debug, Deserialize)]
pub(crate) struct PhaseReplyRow {
    pub(crate) phase: PhaseRow,
    pub(crate) mode:  Option<ModeRow>,
    pub(crate) modes: Vec<ModeRow>,
}

/// Decode the RON body of a `Ran` outcome, or say which outcome came back instead.
pub(crate) fn decoded_ran(reply: &McpResponse) -> Result<PhaseReplyRow, TestError> {
    let McpResponse::Outcome(CommandOutcome::Ran { reply: body, .. }) = reply else {
        return Err(format!("expected a Ran outcome for `editor.phase`, got {reply:?}").into());
    };
    Ok(ron::de::from_str::<PhaseReplyRow>(body.as_str())?)
}

/// The reported phase must name the editor state the frame that answered was in.
pub(crate) fn assert_phase_is(row: PhaseRow, expected: EditorState) {
    assert_eq!(
        format!("{row:?}"),
        format!("{expected:?}"),
        "the reply names the editor's own state on the frame that answered, not a phase of its \
         own invention",
    );
}

/// The reported tab list must be the editor's own tab bar, in tab-bar order.
pub(crate) fn assert_modes_are_the_tab_order(modes: &[ModeRow]) {
    let expected: Vec<String> = EditorMode::TAB_ORDER
        .iter()
        .map(|mode| format!("{mode:?}"))
        .collect();
    let reported: Vec<String> = modes.iter().map(|mode| format!("{mode:?}")).collect();
    assert_eq!(
        reported, expected,
        "the reply lists every tab the editor offers, in the order the tab bar draws them",
    );
}
