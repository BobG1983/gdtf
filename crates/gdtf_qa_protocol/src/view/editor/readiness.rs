//! [`EditorReadinessNet`] — the content editor's lifecycle readiness (GTW-805).

use serde::{Deserialize, Serialize};

/// The editor's top-level lifecycle **state** — the wire mirror of the editor's own
/// `EditorState`.
///
/// The editor's counterpart of [`AppStateNet`](crate::view::AppStateNet), and the reason
/// every editor query reply carries one: the editor boots into
/// [`Load`](Self::Load) (the asset pass that resolves the tile roles and the content
/// registries) and only reaches [`Editing`](Self::Editing) afterwards, and which of the two
/// produced an answer changes what that answer means. The authoring model resources — the
/// active mode, the session, the drafts — are inserted on entering `Editing` and exist only
/// there, so the topics reading them cannot be answered during `Load`. The
/// content-integrity report is NOT one of those: the editor host builds it into the app
/// before the first frame, so the validation topic IS answered during `Load`, about an editor
/// whose checks may not have run yet — which is what its own `checks_complete` flag reports
/// (GTW-879). A client that starts driving the editor without reading this races the asset
/// pass. An independent serde enum mirroring the two `EditorState` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorReadinessNet {
    /// The asset pass is still running — the authoring model (mode, session, drafts) does
    /// not exist yet, and whatever IS answerable describes a half-built editor (the
    /// editor's `Load`).
    Load,
    /// The authoring scene is up and its model resources are live (the editor's
    /// `Editing`).
    Editing,
}

impl EditorReadinessNet {
    /// Whether the editor has finished its asset pass and is ready to be driven — `true`
    /// exactly in [`Editing`](Self::Editing).
    ///
    /// The one-call answer to "may I start injecting?", so a polling client does not have
    /// to re-derive it from the variant every loop.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Editing)
    }
}
