//! [`SelectionView`] — the current ganger selection (GTW-734).

use serde::{Deserialize, Serialize};

use crate::ids::GangerToken;

/// The current ganger **selection** — the wire mirror of the input `SelectedShooter`.
///
/// The actor most classic [`NetIntent`](crate::intent::NetIntent) variants act through
/// (a client [`Select`](crate::intent::NetIntent::Select)s first). `None` when nothing
/// is selected. Serde default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SelectionView {
    /// The selected ganger's token, or `None` when nothing is selected.
    pub selected: Option<GangerToken>,
}

impl SelectionView {
    /// Build a selection view from the optional selected token.
    #[must_use]
    pub const fn new(selected: Option<GangerToken>) -> Self {
        Self { selected }
    }
}
