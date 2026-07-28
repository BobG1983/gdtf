//! [`readiness`] — the editor's lifecycle readiness on the wire (GTW-805).

use gdtf_qa_protocol::view::EditorReadinessNet;

use crate::EditorState;

/// Map the editor's own [`EditorState`] onto its wire mirror.
///
/// A wildcard-free `match`, so a third editor state must decide what it reports rather than
/// silently reading as "ready". This is the fact that rides on EVERY editor query reply,
/// because the answer a topic gives depends on the phase it was read in: the mode, session
/// and draft resources exist only in [`Editing`](EditorState::Editing), and the topics that
/// ARE answerable during [`Load`](EditorState::Load) answer about a half-built editor — an
/// empty finding list during `Load` means "not checked yet", not "clean". A client that could
/// not see the phase an answer came from would read a mid-`Load` snapshot as a finished one.
pub(super) const fn readiness(state: &EditorState) -> EditorReadinessNet {
    match state {
        EditorState::Load => EditorReadinessNet::Load,
        EditorState::Editing => EditorReadinessNet::Editing,
    }
}
