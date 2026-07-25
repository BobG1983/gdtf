//! [`EditorModeView`] — the active authoring mode (GTW-805).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The editor's active **authoring mode** — the wire mirror of the editor's `EditorMode`.
///
/// One variant per Workbench tab, in tab order. An independent closed serde enum (never a
/// leak of the editor type): a new authoring mode forces a new variant here rather than
/// silently reading as its neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorModeNet {
    /// TERRAIN authoring — a terrain definition.
    Terrain,
    /// THEME authoring — a theme assembled from the terrain library.
    Theme,
    /// PREFAB authoring — the painted prefab map.
    Prefab,
    /// GANG authoring — a gang roster.
    Gang,
    /// ARMOR authoring — an armor suit's per-body-part pieces.
    Armor,
    /// INJURY authoring — an injury def and its weighting rows.
    Injury,
    /// SPRITE authoring — a sprite def.
    Sprite,
    /// ATTACHMENT authoring — a weapon attachment item.
    Attachment,
    /// WEAPON authoring — a ranged weapon spec.
    Weapon,
    /// MELEE-WEAPON authoring — a melee weapon spec.
    MeleeWeapon,
}

/// The mode's **tab label** — the caption on its Workbench tab (e.g. `"PREFAB"`).
///
/// So a client can name the mode the way the on-screen tab does. A named newtype over the
/// caption string (no-bare-types), serde-transparent; `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorModeLabelNet(String);

impl EditorModeLabelNet {
    /// Build a mode label from its tab caption.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// The mode's **position in the tab order**, left to right, zero-based.
///
/// A private-inner newtype over `u8` (no-bare-types), serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorTabIndexNet(u8);

impl EditorTabIndexNet {
    /// Build a tab index from its zero-based position in the tab order.
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index)
    }
}

/// The **mode snapshot** — which authoring mode is active, by variant, caption and tab
/// position.
///
/// The reply payload of [`EditorQueryKind::Mode`](crate::view::EditorQueryKind::Mode).
/// Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorModeView {
    /// The active mode.
    pub active:    EditorModeNet,
    /// Its on-screen tab caption.
    pub label:     EditorModeLabelNet,
    /// Its zero-based position in the tab order.
    pub tab_index: EditorTabIndexNet,
}

impl EditorModeView {
    /// Build a mode snapshot from the active mode, its caption and its tab position.
    #[must_use]
    pub const fn new(
        active: EditorModeNet,
        label: EditorModeLabelNet,
        tab_index: EditorTabIndexNet,
    ) -> Self {
        Self {
            active,
            label,
            tab_index,
        }
    }
}
