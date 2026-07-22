//! The GTW-582 validation-pass PLUMBING — the report resource, the
//! `Check`/`Publish` sets and their hand-off markers, the consolidated publish
//! system, and the one-call registration ext (see the parent [`report`](super)
//! module doc for the contract).

use bevy::{
    app::{App, Update},
    ecs::{
        resource::Resource,
        schedule::{
            IntoScheduleConfigs, SystemCondition, SystemSet,
            common_conditions::{not, resource_exists},
        },
        system::{Commands, Res, ScheduleSystem},
    },
    log::{info, warn},
};

use super::finding::ContentFinding;

/// The accumulated content-integrity findings — the ONE report the unified
/// dangling-reference contract publishes at the end of `Load` and last-resort
/// fallbacks keep appending to afterwards (GTW-582 C2/C5).
///
/// Init'd by [`ContentValidationAppExt`] (and by
/// [`register_content_family`](crate::ContentFamilyAppExt::register_content_family),
/// so the per-file salvage can record) and NEVER removed — it persists past `Load`
/// so post-`Load` findings (procgen fallbacks) land on the same record and tests
/// can read it whole.
#[derive(Resource, Debug, Default)]
pub struct ContentIntegrityReport(Vec<ContentFinding>);

impl ContentIntegrityReport {
    /// Append one finding to the report.
    pub fn record(&mut self, finding: ContentFinding) {
        self.0.push(finding);
    }

    /// Every finding recorded so far, in record order.
    #[must_use]
    pub fn findings(&self) -> &[ContentFinding] {
        &self.0
    }

    /// Whether the report holds no findings — the shipped-content invariant.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.0.is_empty()
    }

    /// How many findings the report holds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the report holds no findings (the `len`-companion emptiness probe).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The two phases of the end-of-`Load` validation pass. The host schedules its
/// per-edge check systems into [`Check`](ContentValidationSet::Check) (via
/// [`ContentValidationAppExt::register_reference_check`]) and gates THAT set on
/// its own "every registry has resolved, not yet checked" run condition;
/// [`Publish`](ContentValidationSet::Publish) is configured once, here: ordered
/// after `Check` and gated on [`ContentChecksComplete`] — the marker the
/// `Check`-set sentinel stamps — so the consolidated report can NEVER print in a
/// frame the checks did not run in (a set's run condition is evaluated once per
/// schedule run, so the `Check` members run all-or-none; the marker hand-off
/// makes the publish wait for the frame AFTER they all ran, immune to
/// mid-frame command-application timing — `bevy-traps.md` #3).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentValidationSet {
    /// The per-edge reference checks — each appends its findings to the
    /// [`ContentIntegrityReport`] — plus the [`ContentChecksComplete`] sentinel.
    Check,
    /// The one consolidated publish — logs the report and stamps
    /// [`ContentValidationDone`].
    Publish,
}

/// PRIVATE install-once marker for
/// [`ContentValidationAppExt::init_content_validation`] — present exactly when
/// the validation plumbing (sets + sentinel + publish) has been installed on
/// this app. Deliberately NOT the [`ContentIntegrityReport`]: the report is
/// also init'd by family registration, so it cannot key the plumbing install.
#[derive(Resource, Debug, Default)]
struct ContentValidationPlumbing;

/// Marker resource stamped by the [`ContentValidationSet::Check`] sentinel
/// ([`mark_content_checks_complete`]) the frame every per-edge check ran —
/// [`ContentValidationSet::Publish`] gates on it, so the consolidated report
/// prints strictly AFTER the checks. Never removed (the pass runs once).
#[derive(Resource, Debug, Default)]
pub struct ContentChecksComplete;

/// Marker resource stamped by [`publish_content_integrity_report`] once the
/// unified validation pass has run — the host's `Load` exit gate requires it, so
/// the pass is EXPLICITLY ordered before the `Load → Intro` transition. Never
/// removed (validation runs once per app run).
#[derive(Resource, Debug, Default)]
pub struct ContentValidationDone;

/// `Update` (in [`ContentValidationSet::Check`]): stamp [`ContentChecksComplete`]
/// — the sentinel that hands the pass from the checks to the publish. It shares
/// the `Check` set's ONE run-condition evaluation, so the marker is stamped
/// exactly when the per-edge checks ran (all-or-none set semantics).
pub fn mark_content_checks_complete(mut commands: Commands) {
    commands.insert_resource(ContentChecksComplete);
}

/// `Update` (in [`ContentValidationSet::Publish`]): emit the ONE consolidated
/// loud report and stamp [`ContentValidationDone`].
///
/// Zero findings logs a one-line `info!` (the clean bill); any findings log ONE
/// `warn!` block listing every finding (the loud, non-fatal contract — the
/// `audit_unweighted_injuries` precedent generalized). The done-marker is stamped
/// UNCONDITIONALLY, so validation can never strand the host's presence-gated
/// `Load` flow (the no-strand guarantee).
///
/// Takes the report as `Option<Res<…>>` (bevy-traps rule 1, belt-and-braces —
/// the registering ext init's it, but a foreign host might not).
pub fn publish_content_integrity_report(
    report: Option<Res<ContentIntegrityReport>>,
    mut commands: Commands,
) {
    match report.as_deref() {
        Some(report) if !report.is_clean() => {
            let lines: Vec<String> = report
                .findings()
                .iter()
                .map(|finding| format!("  - {finding}"))
                .collect();
            warn!(
                "content reference contract: {} finding(s) at end of Load:\n{}",
                report.len(),
                lines.join("\n"),
            );
        }
        Some(_) => {
            info!("content reference contract: every authored reference resolves (0 findings)");
        }
        None => {}
    }
    commands.insert_resource(ContentValidationDone);
}

/// One-call installation of the unified validation pass — the GTW-582 registration
/// trait (gate directive P1: a new family's validation is ONE
/// [`register_reference_check`](Self::register_reference_check) call, never an
/// edit to a shared walker).
pub trait ContentValidationAppExt {
    /// Install the validation plumbing: the [`ContentIntegrityReport`] resource,
    /// the `Check → Publish` set ordering in `Update`, the
    /// [`mark_content_checks_complete`] sentinel (in `Check`), and the
    /// [`publish_content_integrity_report`] system (in `Publish`, gated on
    /// [`ContentChecksComplete`] + not-yet-[`ContentValidationDone`]). Idempotent
    /// — safe to call once from the host plugin and implicitly from every check
    /// registration.
    ///
    /// WHEN the checks run stays the HOST's: gate
    /// [`ContentValidationSet::Check`] with `configure_sets(Update, …run_if(…))`
    /// on the host's own "all registries resolved, not yet checked" condition
    /// (include `not(resource_exists::<ContentChecksComplete>)` so the checks
    /// run exactly once).
    fn init_content_validation(&mut self) -> &mut Self;

    /// Register one per-edge reference-check system into
    /// [`ContentValidationSet::Check`]. The system reads the registries its edge
    /// spans and appends [`ContentFinding`]s to the
    /// [`ContentIntegrityReport`].
    fn register_reference_check<M>(
        &mut self,
        check: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;
}

impl ContentValidationAppExt for App {
    fn init_content_validation(&mut self) -> &mut Self {
        // Idempotence keys on the PRIVATE plumbing marker, NOT the report:
        // `register_content_family` also init's the report (the salvage records
        // into it), so keying on the report would skip installing the sets +
        // publish system in any app that registered a family first.
        if self
            .world()
            .get_resource::<ContentValidationPlumbing>()
            .is_none()
        {
            self.insert_resource(ContentValidationPlumbing);
            self.init_resource::<ContentIntegrityReport>();
            self.configure_sets(
                Update,
                ContentValidationSet::Publish
                    .after(ContentValidationSet::Check)
                    // The publish waits for the checks' sentinel and runs once —
                    // both markers are monotonic, so this gating is immune to
                    // mid-frame command-application timing (bevy-traps #3).
                    .run_if(
                        resource_exists::<ContentChecksComplete>
                            .and_then(not(resource_exists::<ContentValidationDone>)),
                    ),
            );
            self.add_systems(
                Update,
                mark_content_checks_complete.in_set(ContentValidationSet::Check),
            );
            self.add_systems(
                Update,
                publish_content_integrity_report.in_set(ContentValidationSet::Publish),
            );
        }
        self
    }

    fn register_reference_check<M>(
        &mut self,
        check: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.init_content_validation();
        self.add_systems(Update, check.in_set(ContentValidationSet::Check));
        self
    }
}
