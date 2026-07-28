//! [`EditorQaModel`] — the one read of the editor's model the query service answers from
//! (GTW-805).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_assets::{ContentChecksComplete, ContentIntegrityReport};
use gdtf_qa_protocol::view::EditorReadinessNet;

use super::{draft::EditorDraftModel, readiness::readiness};
use crate::{EditorMode, EditorState, MapEditorSession};

/// Everything the editor query topics read: the editor's own lifecycle state plus the
/// authoring model resources.
///
/// One [`SystemParam`] the request drain takes, rather than a widening parameter list on the
/// router (bevy-traps: a fn-system's parameters are capped at 16, and this is one concern).
/// Every model resource is `Option<Res<…>>` because none of them is guaranteed present: the
/// mode / session / draft resources are state-scoped to `Editing` (bevy-traps #1) and do not
/// exist during the editor's `Load` asset pass, while the report exists from app-build time
/// and the checks-complete marker lands mid-`Load`. Whatever is absent is REPORTED (the topic
/// is not offered), never papered over with an empty view.
///
/// [`State<EditorState>`] is a plain `Res`: the drain's own run condition is
/// `resource_exists::<State<EditorState>>`, so it is always present when this runs.
#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorQaModel<'w> {
    /// The editor's lifecycle state.
    state:           Res<'w, State<EditorState>>,
    /// The active authoring mode.
    mode:            Option<Res<'w, EditorMode>>,
    /// The shared authoring session (theme / floor / grid / paint tile).
    session:         Option<Res<'w, MapEditorSession>>,
    /// The accumulated content-integrity findings.
    report:          Option<Res<'w, ContentIntegrityReport>>,
    /// Present once the per-edge content checks have run.
    checks_complete: Option<Res<'w, ContentChecksComplete>>,
    /// The per-mode authoring drafts.
    drafts:          EditorDraftModel<'w>,
}

impl EditorQaModel<'_> {
    /// The editor's lifecycle readiness — the fact every editor query reply carries.
    pub(in crate::net_qa) fn readiness(&self) -> EditorReadinessNet {
        readiness(self.state.get())
    }

    /// The active authoring mode, or [`None`] outside `Editing`.
    pub(in crate::net_qa) fn mode(&self) -> Option<EditorMode> {
        self.mode.as_deref().copied()
    }

    /// The authoring session, or [`None`] outside `Editing`.
    pub(in crate::net_qa) fn session(&self) -> Option<&MapEditorSession> {
        self.session.as_deref()
    }

    /// The content-integrity report, or [`None`] before the validation plumbing installs it.
    pub(in crate::net_qa) fn report(&self) -> Option<&ContentIntegrityReport> {
        self.report.as_deref()
    }

    /// Whether the per-edge content checks have run in this session.
    pub(in crate::net_qa) const fn checks_complete(&self) -> bool {
        self.checks_complete.is_some()
    }

    /// The per-mode authoring drafts.
    pub(in crate::net_qa) const fn drafts(&self) -> &EditorDraftModel<'_> {
        &self.drafts
    }
}
