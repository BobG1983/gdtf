//! Every editor write needs the authoring scene, so each refuses the Load pass alike.

use gdtf_qa_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

use crate::net_qa::{facts::EditorFacts, wire::EditorPhaseNet};

/// Available once the authoring scene is live; `WrongState` while the editor still loads.
pub(in crate::net_qa::commands::write) fn only_while_editing(
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
