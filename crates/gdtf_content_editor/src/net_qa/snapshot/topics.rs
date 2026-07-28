//! The topic-availability filter + the options reply (GTW-805).
//!
//! ADR 0007's live discovery signal for the editor family: the SAME predicate decides both
//! what [`GetEditorQueryOptions`](gdtf_qa_protocol::envelope::QaRequest::GetEditorQueryOptions)
//! advertises and whether a
//! [`QueryEditor`](gdtf_qa_protocol::envelope::QaRequest::QueryEditor) is answered, so the two
//! can never disagree — the game router's `request_available` pattern, applied to
//! [`EditorQueryKind`] instead of `RequestKindNet`.

use gdtf_qa_protocol::view::{EditorQueryKind, EditorQueryOptionsView, EditorQueryTopicView};

use super::model::EditorQaModel;

/// Whether the editor will answer `kind` right now.
///
/// A wildcard-free `match` (ADR 0007's drift guard): a topic added to [`EditorQueryKind`]
/// fails this match to compile until it states when it is live, so the static topic list and
/// the live-availability list cannot drift apart.
///
/// Readiness is always answerable — it is what a client polls DURING the editor's `Load`
/// asset pass to learn when the editor is ready. Every OTHER topic is offered exactly when
/// the resource it reads is present, and that is NOT the same window for all of them
/// (GTW-879, which corrected this doc and the test that had copied its earlier claim):
///
/// - `Mode` / `Session` / `Draft` read model resources the editor inserts `OnEnter(Editing)`
///   and removes `OnExit(Editing)` (bevy-traps #1), so they appear only in `Editing`.
/// - `Validation` reads [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport),
///   which every content-family registration and the validation plumbing `init_resource`
///   while the app is still being BUILT (`load/injuries.rs`, `gdtf_assets`'s family
///   registration + `init_content_validation`). So it is offered from the editor's first
///   frame, `Load` included. That is the DECIDED behavior and the honest one: the report
///   genuinely exists, and its `checks_complete` flag is precisely what tells a client the
///   checks have not run yet — an empty finding list during `Load` means "not checked", not
///   "clean". Scoping the report to `Editing` to keep the older doc true would have hidden a
///   resource that is really there.
///
/// Either way a topic is never advertised unless the answer really can be produced.
pub(in crate::net_qa) fn topic_available(kind: EditorQueryKind, model: &EditorQaModel) -> bool {
    match kind {
        EditorQueryKind::Readiness => true,
        EditorQueryKind::Mode => model.mode().is_some(),
        EditorQueryKind::Session => model.session().is_some(),
        EditorQueryKind::Draft => model.mode().is_some_and(|mode| model.drafts().has(mode)),
        EditorQueryKind::Validation => model.report().is_some(),
    }
}

/// The options reply: the editor's readiness plus every topic it will service right now, in
/// topic-declaration order.
pub(in crate::net_qa) fn options_view(model: &EditorQaModel) -> EditorQueryOptionsView {
    let topics = EditorQueryKind::ALL
        .into_iter()
        .filter(|kind| topic_available(*kind, model))
        .map(EditorQueryTopicView::offered)
        .collect();
    EditorQueryOptionsView::new(model.readiness(), topics)
}
