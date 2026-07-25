//! The two editor query REPLIES — the options menu and the per-topic answer (GTW-805).

use serde::{Deserialize, Serialize};

use crate::view::editor::{
    draft::EditorDraftView, mode::EditorModeView, readiness::EditorReadinessNet,
    session::EditorSessionView, topic::EditorQueryTopicView, validation::EditorValidationView,
};

/// One topic's **answer** — one variant per
/// [`EditorQueryKind`](crate::view::EditorQueryKind).
///
/// Each variant is a small, purpose-scoped view rather than a slice of one large editor
/// blob (ADR 0007): a client asking about the active mode is not handed the whole draft.
/// The wildcard-free `match` a host writes to build one keeps the answer vocabulary in
/// lock-step with the topic vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorQueryView {
    /// The lifecycle readiness on its own (the
    /// [`Readiness`](crate::view::EditorQueryKind::Readiness) topic).
    Readiness(EditorReadinessNet),
    /// The active authoring mode (the [`Mode`](crate::view::EditorQueryKind::Mode) topic).
    Mode(EditorModeView),
    /// The authoring session's selections (the
    /// [`Session`](crate::view::EditorQueryKind::Session) topic).
    Session(EditorSessionView),
    /// The active mode's draft fields (the [`Draft`](crate::view::EditorQueryKind::Draft)
    /// topic).
    Draft(EditorDraftView),
    /// The content-validation state (the
    /// [`Validation`](crate::view::EditorQueryKind::Validation) topic).
    Validation(EditorValidationView),
}

/// The reply to a [`QueryEditor`](crate::envelope::QaRequest::QueryEditor) — the topic's
/// answer, with the editor's lifecycle readiness alongside it.
///
/// Readiness rides on EVERY editor query reply, not only on the
/// [`Readiness`](crate::view::EditorQueryKind::Readiness) topic. The editor boots into
/// `Load` (its asset pass) and only then reaches `Editing`, and every model resource a topic
/// reads is scoped to `Editing`; a client that cannot see which phase produced an answer has
/// no way to tell "the editor is not up yet" from "the editor is up and the answer is
/// this". Carrying it here is the editor's counterpart of the game's
/// [`AppFlowView`](crate::view::AppFlowView) poll. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorQueryReply {
    /// Where the editor was in its lifecycle when this answer was produced.
    pub readiness: EditorReadinessNet,
    /// The topic's answer.
    pub view:      EditorQueryView,
}

impl EditorQueryReply {
    /// Build a query reply from the editor's readiness and the topic's answer.
    #[must_use]
    pub const fn new(readiness: EditorReadinessNet, view: EditorQueryView) -> Self {
        Self { readiness, view }
    }
}

/// The reply to a
/// [`GetEditorQueryOptions`](crate::envelope::QaRequest::GetEditorQueryOptions) — the topics
/// the editor will service RIGHT NOW, with its lifecycle readiness.
///
/// The live half of ADR 0007's two discovery signals (the static half is the MCP layer's
/// topic schema): during the editor's `Load` pass the model-backed topics are absent from
/// [`topics`](Self::topics), so a client polls this first and acts only on what it lists —
/// the same contract [`AppFlowView::available`](crate::view::AppFlowView::available) has on
/// the game side. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorQueryOptionsView {
    /// Where the editor is in its lifecycle.
    pub readiness: EditorReadinessNet,
    /// The topics serviceable right now, in topic-declaration order.
    pub topics:    Vec<EditorQueryTopicView>,
}

impl EditorQueryOptionsView {
    /// Build an options reply from the editor's readiness and the live topic list.
    #[must_use]
    pub const fn new(readiness: EditorReadinessNet, topics: Vec<EditorQueryTopicView>) -> Self {
        Self { readiness, topics }
    }
}
