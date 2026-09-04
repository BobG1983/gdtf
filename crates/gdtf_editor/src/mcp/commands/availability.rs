//! Every editor command that needs the authoring scene, or one tab of it, refuses alike.

use cobalt_mcp_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

use crate::mcp::{
    facts::EditorFacts,
    wire::{EditorModeNet, EditorPhaseNet},
};

/// Available once the authoring scene is live; `WrongState` while the editor still loads.
pub(in crate::mcp::commands) fn only_while_editing(
    facts: EditorFacts,
    note: RefusalNote,
) -> CommandAvailability {
    match facts.phase() {
        EditorPhaseNet::Editing => CommandAvailability::Available,
        EditorPhaseNet::Load => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note,
        },
    }
}

/// The phase check first, then the open tab must be the one the command names.
pub(in crate::mcp::commands) fn only_on_the_tab(
    facts: EditorFacts,
    tab: EditorModeNet,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    match only_while_editing(facts, phase_note) {
        CommandAvailability::Available if facts.mode() == Some(tab) => {
            CommandAvailability::Available
        }
        CommandAvailability::Available => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: tab_note,
        },
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

/// The phase check first, then the open tab must be Theme.
pub(in crate::mcp::commands) fn only_on_the_theme_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    only_on_the_tab(facts, EditorModeNet::Theme, phase_note, tab_note)
}

/// The phase check first, then the open tab must be Prefab.
pub(in crate::mcp::commands) fn only_on_the_prefab_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    only_on_the_tab(facts, EditorModeNet::Prefab, phase_note, tab_note)
}

/// The phase check first, then the open tab must be Injury.
pub(in crate::mcp::commands) fn only_on_the_injury_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    only_on_the_tab(facts, EditorModeNet::Injury, phase_note, tab_note)
}

/// The form-mode check first, then that mode's own draft must be in the world.
pub(in crate::mcp::commands) fn only_in_a_form_mode_with_its_draft(
    facts: EditorFacts,
    phase_note: RefusalNote,
    prefab_note: RefusalNote,
    draft_note: RefusalNote,
) -> CommandAvailability {
    match only_in_a_form_mode(facts, phase_note, prefab_note) {
        CommandAvailability::Available if *facts.draft() => CommandAvailability::Available,
        CommandAvailability::Available => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: draft_note,
        },
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

/// The phase check first, then the open tab must be one of the ten form modes.
pub(in crate::mcp::commands) fn only_in_a_form_mode(
    facts: EditorFacts,
    phase_note: RefusalNote,
    prefab_note: RefusalNote,
) -> CommandAvailability {
    match only_while_editing(facts, phase_note) {
        CommandAvailability::Available
            if matches!(facts.mode(), Some(EditorModeNet::Prefab) | None) =>
        {
            CommandAvailability::Unavailable {
                code: UnavailableCode::WrongState,
                note: prefab_note,
            }
        }
        available @ CommandAvailability::Available => available,
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

#[cfg(test)]
mod test {
    use cobalt_mcp_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

    use super::only_on_the_tab;
    use crate::mcp::{
        facts::{DraftInWorld, EditorFacts},
        wire::{EditorModeNet, EditorPhaseNet},
    };

    const PHASE_NOTE: RefusalNote = RefusalNote::from_static("the authoring scene is not live");

    const TAB_NOTE: RefusalNote = RefusalNote::from_static("that tab is not the open one");

    fn facts(phase: EditorPhaseNet, mode: Option<EditorModeNet>) -> EditorFacts {
        EditorFacts::new(phase, mode, DraftInWorld::new(true))
    }

    #[test]
    fn the_tab_the_command_names_is_available() {
        assert_eq!(
            only_on_the_tab(
                facts(EditorPhaseNet::Editing, Some(EditorModeNet::Armor)),
                EditorModeNet::Armor,
                PHASE_NOTE,
                TAB_NOTE,
            ),
            CommandAvailability::Available,
        );
    }

    #[test]
    fn another_tab_is_refused_with_the_tab_note() {
        assert_eq!(
            only_on_the_tab(
                facts(EditorPhaseNet::Editing, Some(EditorModeNet::Gang)),
                EditorModeNet::Armor,
                PHASE_NOTE,
                TAB_NOTE,
            ),
            CommandAvailability::Unavailable {
                code: UnavailableCode::WrongState,
                note: TAB_NOTE,
            },
            "a client on the wrong tab is told which tab to open, not that the scene is missing",
        );
    }

    #[test]
    fn the_load_pass_is_refused_with_the_phase_note() {
        assert_eq!(
            only_on_the_tab(
                facts(EditorPhaseNet::Load, None),
                EditorModeNet::Armor,
                PHASE_NOTE,
                TAB_NOTE,
            ),
            CommandAvailability::Unavailable {
                code: UnavailableCode::WrongState,
                note: PHASE_NOTE,
            },
            "the phase is checked first, so a client waiting on the scene is told that rather \
             than being sent to open a tab that does not exist yet",
        );
    }
}
