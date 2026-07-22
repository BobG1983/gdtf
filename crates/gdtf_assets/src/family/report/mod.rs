//! The GTW-582 **content-integrity report** — the ONE unified dangling-reference
//! contract's vocabulary and plumbing (Q3 ruling, 2026-07-02).
//!
//! The content reference graph (situations → gangs → members → equipment keys,
//! weapons → attachment keys, themes/prefabs → terrain-def UUIDs, …) used to fail
//! four different ways: an abort-first setup error, a silent resolution drop, a
//! warn-and-skip, and a silent nil-sentinel fallback. This module gives every one
//! of those modes ONE place to surface: a [`ContentIntegrityReport`] resource that
//! accumulates typed [`ContentFinding`]s, a pair of [`ContentValidationSet`]s the
//! host schedules its per-edge check systems into, and a
//! [`publish_content_integrity_report`] system that emits ONE consolidated loud
//! log at the end of `Load` and stamps [`ContentValidationDone`].
//!
//! Validation NEVER strands the host's `Load` flow: the publish system stamps the
//! done-marker unconditionally — findings are LOUD, not fatal (the
//! `audit_unweighted_injuries` precedent, generalized).
//!
//! Submodules by concern (wiring only here):
//!
//! - [`finding`] — the finding vocabulary: the label newtypes, the
//!   [`ReferenceKeyScheme`], and [`ContentFinding`] itself.
//! - [`pass`] — the pass plumbing: the report resource, the `Check`/`Publish`
//!   sets + markers, the publish system, and the registration ext.
//!
//! # Who writes findings
//!
//! - the host's per-edge check systems, registered through
//!   [`ContentValidationAppExt::register_reference_check`] (one hook per edge —
//!   family N+1 registers one more system, never edits a shared walker);
//! - the content-family machinery's per-file fail-closed salvage
//!   ([`resolve_content_family`](crate::resolve_content_family)), which records a
//!   [`ContentFinding::MalformedFile`] for each member that failed to parse;
//! - post-`Load` last-resort fallbacks (e.g. the procgen empty-board fallback),
//!   which `warn!` at their own site and append here so the degradation is on
//!   the record, never silent.

mod finding;
mod pass;

pub use finding::{
    ContentFinding, FindingDetail, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
pub use pass::{
    ContentChecksComplete, ContentIntegrityReport, ContentValidationAppExt, ContentValidationDone,
    ContentValidationSet, mark_content_checks_complete, publish_content_integrity_report,
};
