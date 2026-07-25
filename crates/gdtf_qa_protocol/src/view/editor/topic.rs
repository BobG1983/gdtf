//! [`EditorQueryTopicView`] — one offered topic + its one-line description (GTW-805).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::view::editor::kind::EditorQueryKind;

/// A topic's **one-line description** — what asking it answers.
///
/// So a client (or a language model driving one) can choose a topic from the live options
/// list without consulting the protocol source. A named newtype over the description string
/// (no-bare-types), serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorTopicDescriptionNet(String);

impl EditorTopicDescriptionNet {
    /// Build a topic description from its one-line text.
    #[must_use]
    pub const fn new(description: String) -> Self {
        Self(description)
    }
}

/// One **offered** editor query topic — the kind plus its one-line description.
///
/// The row type of the
/// [`GetEditorQueryOptions`](crate::envelope::QaRequest::GetEditorQueryOptions) reply: the
/// editor lists only the topics it will actually service in its current state, so "what may
/// I ask" and "what will be answered" are one list rather than two that can drift. Serde
/// default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorQueryTopicView {
    /// The topic a client echoes back in a
    /// [`QueryEditor`](crate::envelope::QaRequest::QueryEditor).
    pub kind:        EditorQueryKind,
    /// What asking this topic answers.
    pub description: EditorTopicDescriptionNet,
}

impl EditorQueryTopicView {
    /// Build an offered topic from its kind and description.
    #[must_use]
    pub const fn new(kind: EditorQueryKind, description: EditorTopicDescriptionNet) -> Self {
        Self { kind, description }
    }

    /// Build an offered topic for `kind`, taking its description from
    /// [`describe`](Self::describe).
    ///
    /// The description text lives in the protocol crate — with the topic vocabulary it
    /// describes — so both hosts hand out the same words for the same topic.
    #[must_use]
    pub fn offered(kind: EditorQueryKind) -> Self {
        Self::new(
            kind,
            EditorTopicDescriptionNet::new(Self::describe(kind).to_owned()),
        )
    }

    /// The canonical one-line description of `kind`. A wildcard-free `match`, so a new
    /// topic must be described rather than shipped nameless.
    #[must_use]
    pub const fn describe(kind: EditorQueryKind) -> &'static str {
        match kind {
            EditorQueryKind::Readiness => {
                "Where the editor is in its lifecycle: Load (asset pass) or Editing (ready)."
            }
            EditorQueryKind::Mode => {
                "The active authoring mode — which Workbench tab is showing, and its tab index."
            }
            EditorQueryKind::Session => {
                "The authoring session: selected theme, resolved default floor, grid size, paint \
                 tile."
            }
            EditorQueryKind::Draft => {
                "The active mode's in-progress authoring draft, as named fields."
            }
            EditorQueryKind::Validation => {
                "Authoring-time content validation: whether the checks ran, and every finding."
            }
        }
    }
}
