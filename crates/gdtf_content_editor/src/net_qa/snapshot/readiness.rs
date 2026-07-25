//! [`readiness`] — the editor's lifecycle readiness on the wire (GTW-805).

use gdtf_qa_protocol::view::EditorReadinessNet;

use crate::EditorState;

/// Map the editor's own [`EditorState`] onto its wire mirror.
///
/// A wildcard-free `match`, so a third editor state must decide what it reports rather than
/// silently reading as "ready". This is the fact that rides on EVERY editor query reply: the
/// authoring model exists only in [`Editing`](EditorState::Editing), so a client that cannot
/// see the phase an answer came from would race the `Load` asset pass.
pub(super) const fn readiness(state: &EditorState) -> EditorReadinessNet {
    match state {
        EditorState::Load => EditorReadinessNet::Load,
        EditorState::Editing => EditorReadinessNet::Editing,
    }
}
