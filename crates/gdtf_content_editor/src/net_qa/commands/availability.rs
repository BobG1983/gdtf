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

/// The phase check first, then the open tab must be Terrain.
///
/// Phase first because the tab is absent during Load, and that call carries the phase note.
pub(in crate::net_qa::commands) fn only_on_the_terrain_tab(
    facts: EditorFacts,
    phase_note: RefusalNote,
    tab_note: RefusalNote,
) -> CommandAvailability {
    match only_while_editing(facts, phase_note) {
        CommandAvailability::Available if matches!(facts.mode(), Some(EditorModeNet::Terrain)) => {
            CommandAvailability::Available
        }
        CommandAvailability::Available => CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: tab_note,
        },
        refused @ CommandAvailability::Unavailable { .. } => refused,
    }
}

/// The phase check first, then the open tab must be one of the nine form modes.
///
/// Phase first because the tab is absent during Load, and that call carries the phase note.
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
