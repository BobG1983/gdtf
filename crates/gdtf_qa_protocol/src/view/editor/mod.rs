//! The **editor** query view family — the content editor's read surface (GTW-805).
//!
//! The editor half of ADR 0007's per-family query recipe: instead of one monolithic editor
//! snapshot, a client asks
//! [`GetEditorQueryOptions`](crate::envelope::QaRequest::GetEditorQueryOptions) for the
//! topics that are live right now, then
//! [`QueryEditor`](crate::envelope::QaRequest::QueryEditor) for ONE
//! [`EditorQueryKind`] — and gets back a small, purpose-scoped view for that topic.
//!
//! Both replies carry the editor's [`EditorReadinessNet`]. The editor boots into its `Load`
//! asset pass and only then reaches `Editing`, where the authoring model resources (the
//! active mode, the session, the drafts) live, so a client that cannot see the phase an
//! answer came from has no way to keep its inject loop from racing the asset pass. Not every
//! topic waits for `Editing` — the validation topic reads a report the editor host builds
//! into the app up front and is answered from the first frame, so readiness is also what
//! tells a client that a `Load` answer describes a half-built editor (GTW-879).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`readiness`] — the [`EditorReadinessNet`] lifecycle mirror.
//! - [`kind`] — the [`EditorQueryKind`] topic vocabulary.
//! - [`topic`] — the offered-topic row ([`EditorQueryTopicView`]) + its description.
//! - [`mode`] — the active-mode view + the [`EditorModeNet`] mirror.
//! - [`session`] — the authoring-session view + its key newtypes.
//! - [`draft`] — the active mode's draft-field snapshot.
//! - [`validation`] — the content-validation state + its finding rows.
//! - [`query`] — the two reply shapes and the per-topic [`EditorQueryView`] answer.

pub mod draft;
pub mod kind;
pub mod mode;
pub mod query;
pub mod readiness;
pub mod session;
pub mod topic;
pub mod validation;

pub use draft::{
    EditorDraftFieldNameNet, EditorDraftFieldValueNet, EditorDraftFieldView, EditorDraftView,
};
pub use kind::EditorQueryKind;
pub use mode::{EditorModeLabelNet, EditorModeNet, EditorModeView, EditorTabIndexNet};
pub use query::{EditorQueryOptionsView, EditorQueryReply, EditorQueryView};
pub use readiness::EditorReadinessNet;
pub use session::{EditorSessionView, EditorTerrainKeyNet, EditorThemeKeyNet};
pub use topic::{EditorQueryTopicView, EditorTopicDescriptionNet};
pub use validation::{
    EditorFindingDetailNet, EditorFindingKindNet, EditorFindingSubjectNet, EditorFindingView,
    EditorValidationView, ValidationChecksCompleteNet,
};
