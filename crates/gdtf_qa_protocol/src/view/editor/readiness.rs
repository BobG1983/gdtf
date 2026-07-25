//! [`EditorReadinessNet`] — the content editor's lifecycle readiness (GTW-805).

use serde::{Deserialize, Serialize};

/// The editor's top-level lifecycle **state** — the wire mirror of the editor's own
/// `EditorState`.
///
/// The editor's counterpart of [`AppStateNet`](crate::view::AppStateNet), and the reason
/// every editor query reply carries one: the editor boots into
/// [`Load`](Self::Load) (the asset pass that resolves the tile roles and the content
/// registries) and only reaches [`Editing`](Self::Editing) afterwards, while every model
/// resource a topic reads is scoped to `Editing`. A client that starts driving the editor
/// without reading this races the asset pass. An independent serde enum mirroring the two
/// `EditorState` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorReadinessNet {
    /// The asset pass is still running — the authoring model does not exist yet (the
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
