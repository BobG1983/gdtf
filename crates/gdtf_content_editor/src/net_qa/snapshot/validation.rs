//! The VALIDATION topic — the authoring-time content-integrity state (GTW-805).

use gdtf_assets::{ContentFinding, ContentIntegrityReport};
use gdtf_qa_protocol::view::{
    EditorFindingDetailNet, EditorFindingKindNet, EditorFindingSubjectNet, EditorFindingView,
    EditorValidationView, ValidationChecksCompleteNet,
};

/// The VALIDATION topic's answer: whether the per-edge checks have run, and every finding
/// they recorded, in record order.
///
/// `checks_complete` is on the wire deliberately: before the checks run an empty finding
/// list means "not checked yet", not "clean", and a QA client that cannot tell those apart
/// would read a mid-`Load` snapshot as a pass.
pub(super) fn validation_view(
    report: &ContentIntegrityReport,
    checks_complete: bool,
) -> EditorValidationView {
    EditorValidationView::new(
        ValidationChecksCompleteNet::new(checks_complete),
        report.findings().iter().map(finding_view).collect(),
    )
}

/// One finding on the wire — its family, what it is about, and its detail text.
///
/// A wildcard-free `match`, so a fourth failure family must decide how it reports rather
/// than reading as a dangling reference. The detail reuses each finding's own
/// [`Display`](core::fmt::Display) clause where it is the whole story, so the wire text and
/// the editor's log say the same thing.
fn finding_view(finding: &ContentFinding) -> EditorFindingView {
    let (kind, subject, detail) = match finding {
        ContentFinding::DanglingRef {
            referrer,
            target,
            family,
            scheme,
        } => (
            EditorFindingKindNet::DanglingRef,
            (**referrer).clone(),
            format!("unresolved `{}` in {} ({scheme})", &***target, &***family),
        ),
        ContentFinding::MalformedFile {
            path,
            family,
            detail,
        } => (
            EditorFindingKindNet::MalformedFile,
            (**path).clone(),
            format!("failed to load into {}: {}", &***family, &***detail),
        ),
        ContentFinding::DegradedFallback { context, detail } => (
            EditorFindingKindNet::DegradedFallback,
            (**context).clone(),
            (**detail).clone(),
        ),
    };
    EditorFindingView::new(
        kind,
        EditorFindingSubjectNet::new(subject),
        EditorFindingDetailNet::new(detail),
    )
}
