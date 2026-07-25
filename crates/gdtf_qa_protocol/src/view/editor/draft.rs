//! [`EditorDraftView`] — the active mode's in-progress authoring draft (GTW-805).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::view::editor::mode::EditorModeNet;

/// A draft field's **name** — what the form calls it (e.g. `"display_name"`, `"kind"`).
///
/// A named newtype over the field-name string (no-bare-types), serde-transparent;
/// `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorDraftFieldNameNet(String);

impl EditorDraftFieldNameNet {
    /// Build a draft field name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// A draft field's **current value**, rendered as the text the form shows or the closed
/// choice the author picked.
///
/// Text rather than a per-field union: the ten authoring forms hold ten unrelated record
/// shapes, and a wire enum spanning all of their field types would mirror the whole sim
/// content schema into the protocol for no gain to a QA client, which is asserting "the
/// field the form shows reads X". Its own type, distinct from
/// [`EditorDraftFieldNameNet`] (no-bare-types rule 3), serde-transparent;
/// `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorDraftFieldValueNet(String);

impl EditorDraftFieldValueNet {
    /// Build a draft field value from its rendered text.
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }
}

/// One **draft field** — its name and current value.
///
/// Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorDraftFieldView {
    /// What the form calls the field.
    pub name:  EditorDraftFieldNameNet,
    /// What it currently reads.
    pub value: EditorDraftFieldValueNet,
}

impl EditorDraftFieldView {
    /// Build a draft field row from its name and current value.
    #[must_use]
    pub const fn new(name: EditorDraftFieldNameNet, value: EditorDraftFieldValueNet) -> Self {
        Self { name, value }
    }
}

/// The **draft snapshot** — which mode's draft this is, and its fields in form order.
///
/// The reply payload of [`EditorQueryKind::Draft`](crate::view::EditorQueryKind::Draft). The
/// mode is repeated here (a client can also read it from
/// [`EditorModeView`](crate::view::EditorModeView)) so a draft reply is self-describing: the
/// field names only make sense against the mode that owns them. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorDraftView {
    /// The mode whose draft this is.
    pub mode:   EditorModeNet,
    /// The draft's fields, in the order the form presents them.
    pub fields: Vec<EditorDraftFieldView>,
}

impl EditorDraftView {
    /// Build a draft snapshot from the owning mode and its field rows.
    #[must_use]
    pub const fn new(mode: EditorModeNet, fields: Vec<EditorDraftFieldView>) -> Self {
        Self { mode, fields }
    }
}
