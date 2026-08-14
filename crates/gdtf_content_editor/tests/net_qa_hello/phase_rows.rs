use gdtf_content_editor::{EditorMode, EditorState};
use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};
use serde::Deserialize;

use crate::support::TestError;

/// A client's own reading of the editor's mode tab, decoded from the wire by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ModeRow {
    Terrain,
    Theme,
    Prefab,
    Gang,
    Armor,
    Injury,
    Sprite,
    Attachment,
    Weapon,
    MeleeWeapon,
}

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
pub(crate) fn decoded_ran(reply: &QaResponse) -> Result<PhaseReplyRow, TestError> {
    let QaResponse::Outcome(CommandOutcome::Ran { reply: body, .. }) = reply else {
        return Err(format!("expected a Ran outcome for `editor.phase`, got {reply:?}").into());
    };
    Ok(ron::de::from_str::<PhaseReplyRow>(body.as_str())?)
}

/// The reported phase must name the editor state the app was in when it answered.
pub(crate) fn assert_phase_names_the_live_state(row: PhaseRow, live: Option<&EditorState>) {
    let Some(live) = live else {
        unreachable!("the editor's state machine must exist on the frame that answered");
    };
    assert_eq!(
        format!("{row:?}"),
        format!("{live:?}"),
        "the reply names the editor's own live state, not a phase of its own invention",
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
