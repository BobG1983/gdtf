//! [`EditorQueryKind`] — the editor family's closed list of query TOPICS (GTW-805).

use serde::{Deserialize, Serialize};

/// One **topic** a client can ask the content editor about — the payload of a
/// [`QueryEditor`](crate::envelope::QaRequest::QueryEditor).
///
/// The editor half of the ADR 0007 two-variant recipe: instead of one monolithic editor
/// snapshot, a client asks for exactly the topic it needs, and
/// [`GetEditorQueryOptions`](crate::envelope::QaRequest::GetEditorQueryOptions) tells it
/// which topics are live right now. A small closed serde enum, so the MCP layer can publish
/// the legal topic list as an input schema and the editor's router can match it without a
/// wildcard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorQueryKind {
    /// The lifecycle readiness on its own — `Load` (asset pass running) or `Editing`.
    Readiness,
    /// The active authoring mode (which Workbench tab is showing).
    Mode,
    /// The authoring session: the selected theme, its resolved default floor, the drawable
    /// grid size, and the active paint tile.
    Session,
    /// The active mode's in-progress authoring draft, as named fields.
    Draft,
    /// The authoring-time content validation state: whether the checks have run, and every
    /// finding they recorded.
    Validation,
}

impl EditorQueryKind {
    /// Every editor query topic, in declaration order.
    ///
    /// The static list the editor filters by its current state to answer
    /// [`GetEditorQueryOptions`](crate::envelope::QaRequest::GetEditorQueryOptions), and the
    /// list the round-trip suite walks. The per-variant round-trip witness keeps it complete
    /// — a new topic that is not listed here fails that test.
    pub const ALL: [Self; 5] = [
        Self::Readiness,
        Self::Mode,
        Self::Session,
        Self::Draft,
        Self::Validation,
    ];
}
