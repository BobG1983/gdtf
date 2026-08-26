//! Every editor command that needs the authoring scene, or one tab of it, refuses alike.

use gdtf_qa_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

use crate::net_qa::{
    facts::EditorFacts,
    wire::{EditorModeNet, EditorPhaseNet},
};

/// Available once the authoring scene is live; `WrongState` while the editor still loads.
pub(in crate::net_qa::commands) fn only_while_editing(
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

/// The phase check first, then the open tab must be Theme.
pub(in crate::net_qa::commands) fn only_on_the_theme_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    match only_while_editing(facts, phase_note) {
        CommandAvailability::Available if matches!(facts.mode(), Some(EditorModeNet::Theme)) => {
            CommandAvailability::Available
        }
        CommandAvailability::Available => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: tab_note,
        },
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

/// The phase check first, then the open tab must be Prefab.
pub(in crate::net_qa::commands) fn only_on_the_prefab_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    match only_while_editing(facts, phase_note) {
        CommandAvailability::Available if matches!(facts.mode(), Some(EditorModeNet::Prefab)) => {
            CommandAvailability::Available
        }
        CommandAvailability::Available => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: tab_note,
        },
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

/// The phase check first, then the open tab must be Injury.
pub(in crate::net_qa::commands) fn only_on_the_injury_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    match only_while_editing(facts, phase_note) {
        CommandAvailability::Available if matches!(facts.mode(), Some(EditorModeNet::Injury)) => {
            CommandAvailability::Available
        }
        CommandAvailability::Available => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: tab_note,
        },
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

/// The form-mode check first, then that mode's own draft must be in the world.
pub(in crate::net_qa::commands) fn only_in_a_form_mode_with_its_draft(
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
pub(in crate::net_qa::commands) fn only_in_a_form_mode(
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
