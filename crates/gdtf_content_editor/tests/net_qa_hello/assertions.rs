use gdtf_content_editor::{EDITOR_QA_SERVER_NAME, EditorState};
use gdtf_qa_protocol::{
    command::{CommandAvailability, CommandName, CommandOutcome, CommandTiming, UnavailableCode},
    message::{QaError, QaResponse},
};

use crate::client::{
    EDITOR_COMMAND_NAMES, EDITOR_EDITING_ONLY, EDITOR_FORM_TAB_ONLY, EDITOR_INJURY_TAB_ONLY,
    EDITOR_THEME_TAB_ONLY,
};

/// The lifecycle phase a reply says the frame that answered it was in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnsweringPhase {
    /// The editor was still loading its registries.
    Load,
    /// The authoring scene was live.
    Editing,
}

impl AnsweringPhase {
    /// The phase a live editor state names.
    pub(crate) const fn of(state: Option<EditorState>) -> Self {
        match state {
            Some(EditorState::Editing) => Self::Editing,
            _ => Self::Load,
        }
    }
}

pub(crate) fn assert_version_mismatch(reply: &QaResponse) {
    assert!(
        matches!(reply, QaResponse::Error(QaError::VersionMismatch)),
        "expected VersionMismatch for a wrong client version, got {reply:?}",
    );
}

pub(crate) fn assert_unknown_command(reply: &QaResponse) {
    let QaResponse::Outcome(CommandOutcome::Unknown { known }) = reply else {
        unreachable!(
            "expected an Unknown outcome for a command the editor has not built, got {reply:?}"
        );
    };
    for name in EDITOR_COMMAND_NAMES {
        assert!(
            known.contains(&CommandName::from_static(name)),
            "an Unknown outcome lists every name the editor does offer, so a client can fix its \
             spelling in one round trip — `{name}` is missing from {known:?}",
        );
    }
}

/// Assert every published row — name, timing, summary — and the availability `phase` requires.
pub(crate) fn assert_editor_catalogue(reply: &QaResponse, phase: AnsweringPhase) {
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("expected a Catalogue reply to close the window, got {reply:?}");
    };
    assert_eq!(
        *catalogue.host, EDITOR_QA_SERVER_NAME,
        "the catalogue must answer under the EDITOR's own host name, never the game's",
    );
    for name in EDITOR_COMMAND_NAMES {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for `{name}`: {catalogue:?}");
        };
        assert_eq!(
            entry.timing,
            CommandTiming::Immediate,
            "`{name}` answers inside the frame it is claimed in: {entry:?}",
        );
        assert!(
            !entry.summary.as_str().is_empty(),
            "the row carries the one line a client reads to learn what it does: {entry:?}",
        );
        assert_availability(name, &entry.availability, phase);
    }
}

// Which list a name is on decides whether it must be Available in this phase.
fn assert_availability(name: &str, availability: &CommandAvailability, phase: AnsweringPhase) {
    let editing = phase == AnsweringPhase::Editing;
    let tab_scoped = EDITOR_THEME_TAB_ONLY.contains(&name)
        || EDITOR_INJURY_TAB_ONLY.contains(&name)
        || EDITOR_FORM_TAB_ONLY.contains(&name);
    if !tab_scoped && (editing || !EDITOR_EDITING_ONLY.contains(&name)) {
        assert_eq!(
            *availability,
            CommandAvailability::Available,
            "`{name}` must be Available in {phase:?}. A read that needs nothing from the \
             authoring scene answers at every point in the lifecycle, and every command that is \
             not tab-scoped answers once the scene is live",
        );
        return;
    }
    let CommandAvailability::Unavailable { code, note } = availability else {
        unreachable!(
            "`{name}` needs a resource or a tab this phase does not have, so the catalogue must \
             mark it Unavailable: {availability:?}"
        );
    };
    assert_eq!(
        *code,
        UnavailableCode::WrongState,
        "`{name}` is refused for the host's phase or open tab, which is what WrongState names",
    );
    assert!(
        !note.as_str().is_empty(),
        "the refusal carries the line that tells a client what to wait for",
    );
}
