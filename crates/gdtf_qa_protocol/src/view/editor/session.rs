//! [`EditorSessionView`] — the authoring session's selections (GTW-805).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::view::terrain::GridSizeNet;

/// The selected **theme key** — the editor's `ThemeUuid`, in its text form.
///
/// A UUID rather than a name because that is what the theme registry is keyed by; the wire
/// carries its text form so the protocol needs no UUID dependency. A named newtype over the
/// key string (no-bare-types), serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorThemeKeyNet(String);

impl EditorThemeKeyNet {
    /// Build a theme key from its text form.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// A referenced **terrain key** — the editor's `TerrainUuid`, in its text form.
///
/// Its own type rather than a re-use of [`EditorThemeKeyNet`]: a terrain and a theme are
/// different families and are never interchangeable (no-bare-types rule 3). A named newtype
/// over the key string, serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorTerrainKeyNet(String);

impl EditorTerrainKeyNet {
    /// Build a terrain key from its text form.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// The **session snapshot** — the editor's shared theme / default-floor / grid-size / paint
/// tile selection.
///
/// The reply payload of [`EditorQueryKind::Session`](crate::view::EditorQueryKind::Session),
/// and the wire mirror of the editor's `MapEditorSession`. The grid size reuses the battle
/// family's [`GridSizeNet`] deliberately: it is the same measurement of the same kind of
/// grid, in cells. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorSessionView {
    /// The selected theme key.
    pub theme:         EditorThemeKeyNet,
    /// The default-floor terrain resolved from that theme, or `None` when the selected
    /// theme has not resolved one.
    pub default_floor: Option<EditorTerrainKeyNet>,
    /// The drawable-area dimensions.
    pub grid_size:     GridSizeNet,
    /// The active paint tile, or `None` when no palette row has been chosen.
    pub selected_tile: Option<EditorTerrainKeyNet>,
}

impl EditorSessionView {
    /// Build a session snapshot from the selected theme, its resolved default floor, the
    /// drawable-area dimensions, and the active paint tile.
    #[must_use]
    pub const fn new(
        theme: EditorThemeKeyNet,
        default_floor: Option<EditorTerrainKeyNet>,
        grid_size: GridSizeNet,
        selected_tile: Option<EditorTerrainKeyNet>,
    ) -> Self {
        Self {
            theme,
            default_floor,
            grid_size,
            selected_tile,
        }
    }
}
