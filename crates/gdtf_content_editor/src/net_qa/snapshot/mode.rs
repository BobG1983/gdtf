//! The MODE topic — the active authoring mode (GTW-805).

use gdtf_qa_protocol::view::{
    EditorModeLabelNet, EditorModeNet, EditorModeView, EditorTabIndexNet,
};

use crate::EditorMode;

/// Map the editor's [`EditorMode`] onto its wire mirror.
///
/// A wildcard-free `match`, so an eleventh authoring mode must grow a wire variant rather
/// than reading as its neighbour.
pub(in crate::net_qa) const fn mode_to_net(mode: EditorMode) -> EditorModeNet {
    match mode {
        EditorMode::Terrain => EditorModeNet::Terrain,
        EditorMode::Theme => EditorModeNet::Theme,
        EditorMode::Prefab => EditorModeNet::Prefab,
        EditorMode::Gang => EditorModeNet::Gang,
        EditorMode::Armor => EditorModeNet::Armor,
        EditorMode::Injury => EditorModeNet::Injury,
        EditorMode::Sprite => EditorModeNet::Sprite,
        EditorMode::Attachment => EditorModeNet::Attachment,
        EditorMode::Weapon => EditorModeNet::Weapon,
        EditorMode::MeleeWeapon => EditorModeNet::MeleeWeapon,
    }
}

/// The MODE topic's answer: the active mode, its on-screen tab caption, and its position in
/// the tab order — all read from the editor's own mode vocabulary, so the readout cannot
/// drift from what the tabs render.
pub(super) fn mode_view(mode: EditorMode) -> EditorModeView {
    // The tab order is ten entries, so the index always fits a `u8`; a saturating cast keeps
    // that fact from ever becoming a panic if the order grows.
    let index = u8::try_from(mode.tab_index()).unwrap_or(u8::MAX);
    EditorModeView::new(
        mode_to_net(mode),
        EditorModeLabelNet::new(mode.tab_label().to_owned()),
        EditorTabIndexNet::new(index),
    )
}
