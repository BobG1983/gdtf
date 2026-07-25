//! [`EditorValidationView`] — the authoring-time content-validation state (GTW-805).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Whether the editor's content-integrity **checks have run** in this session.
///
/// A finding list is only meaningful once the checks completed: an empty list before that
/// means "not checked yet", not "clean". A private-inner newtype over `bool`
/// (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidationChecksCompleteNet(bool);

impl ValidationChecksCompleteNet {
    /// Build a checks-complete answer — `true` once the per-edge checks have run.
    #[must_use]
    pub const fn new(complete: bool) -> Self {
        Self(complete)
    }
}

/// What KIND of content-integrity finding was recorded — the wire mirror of the sim
/// `ContentFinding` variants.
///
/// A closed, independent serde enum: a new failure family forces a new variant rather than
/// reading as a dangling reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorFindingKindNet {
    /// An authored reference names a key its target family does not hold.
    DanglingRef,
    /// A content file failed to load or parse; its siblings were salvaged.
    MalformedFile,
    /// A degraded last-resort fallback engaged.
    DegradedFallback,
}

/// WHAT the finding is about — the referencing file / key context, or the offending file's
/// path.
///
/// A named newtype over the subject string (no-bare-types), serde-transparent;
/// `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorFindingSubjectNet(String);

impl EditorFindingSubjectNet {
    /// Build a finding subject from its context label.
    #[must_use]
    pub const fn new(subject: String) -> Self {
        Self(subject)
    }
}

/// The finding's **detail** — the unresolved key and the family it should have resolved in,
/// a parse error's text, or a fallback's description.
///
/// Its own type, distinct from [`EditorFindingSubjectNet`] (no-bare-types rule 3),
/// serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorFindingDetailNet(String);

impl EditorFindingDetailNet {
    /// Build a finding detail from its text.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }
}

/// One recorded content-integrity **finding**.
///
/// Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorFindingView {
    /// Which failure family this finding belongs to.
    pub kind:    EditorFindingKindNet,
    /// What the finding is about.
    pub subject: EditorFindingSubjectNet,
    /// The finding's detail text.
    pub detail:  EditorFindingDetailNet,
}

impl EditorFindingView {
    /// Build a finding row from its kind, subject and detail.
    #[must_use]
    pub const fn new(
        kind: EditorFindingKindNet,
        subject: EditorFindingSubjectNet,
        detail: EditorFindingDetailNet,
    ) -> Self {
        Self {
            kind,
            subject,
            detail,
        }
    }
}

/// The **validation snapshot** — whether the checks have run, and every finding they
/// recorded.
///
/// The reply payload of
/// [`EditorQueryKind::Validation`](crate::view::EditorQueryKind::Validation). Serde default
/// shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorValidationView {
    /// Whether the per-edge checks have run in this session.
    pub checks_complete: ValidationChecksCompleteNet,
    /// Every finding recorded so far, in record order.
    pub findings:        Vec<EditorFindingView>,
}

impl EditorValidationView {
    /// Build a validation snapshot from the checks-complete flag and the recorded findings.
    #[must_use]
    pub const fn new(
        checks_complete: ValidationChecksCompleteNet,
        findings: Vec<EditorFindingView>,
    ) -> Self {
        Self {
            checks_complete,
            findings,
        }
    }
}
