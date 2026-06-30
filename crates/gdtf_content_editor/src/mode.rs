//! The Workbench **editor-mode** machine (GTW-474; egui-swept GTW-512) — the [`EditorMode`]
//! resource + the `1`/`2`/`3` mode hotkeys.
//!
//! The editor is a multi-mode "Workbench": exactly one authoring workflow (TERRAIN / THEME /
//! PREFAB) is active at a time, held in the state-scoped [`EditorMode`] resource (NOT a `SubStates`
//! — bevy-traps #5), default [`Prefab`](EditorMode::Prefab).
//!
//! ## GTW-512: the clean swap off `bevy_ui`
//!
//! The pre-egui shell drove the mode through a `bevy_ui` segmented control (`apply_mode_switch`
//! reading `SegmentSelected`), toggled per-mode `Visibility`-containers (`toggle_mode_content`), and
//! synced the tab highlight to a hotkey (`sync_tabs_to_mode`). egui replaces all three: the
//! [`editor_egui_ui`](crate::egui_shell::editor_egui_ui) shell draws the mode TABS as egui
//! `selectable_value`s over this resource (a click sets it), branches the right panel on the active
//! mode in-UI (no `Visibility` containers), and reads the active mode for the status line. So the
//! `bevy_ui` mode systems + the tab / content markers are GONE; this module keeps the [`EditorMode`]
//! enum (+ its tab vocabulary the egui tabs reuse) and the UI-agnostic [`mode_hotkeys`].

use bevy::prelude::*;

/// The active **editor mode** — which authoring workflow the Workbench is showing (GTW-474).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), NOT a `SubStates` (it would trip bevy-traps #5). The egui shell renders one tab
/// per variant and branches its right panel on the active mode.
///
/// A named closed enum (no-bare-types: a mode is a domain value). The variant ORDER is the top-bar
/// tab order, so [`from_tab_index`](EditorMode::from_tab_index) / [`tab_index`](EditorMode::tab_index)
/// map a tab index to a mode and back. The default is [`Prefab`](EditorMode::Prefab) — the painter
/// the editor opened in before the Workbench grew around it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorMode {
    /// TERRAIN authoring — create a terrain definition (`*.terrain_def.ron`).
    Terrain,
    /// THEME authoring — assemble a theme definition (`*.terrain_theme.ron`) from the loaded
    /// terrain library (GTW-475).
    Theme,
    /// PREFAB authoring — paint a prefab with the selected theme (the original editor).
    #[default]
    Prefab,
}

impl EditorMode {
    /// The mode-tab order, left to right — the order the egui shell renders the tabs and the index
    /// order [`from_tab_index`](EditorMode::from_tab_index) maps. THEME sits BETWEEN Terrain and
    /// Prefab (GTW-475): `[TERRAIN | THEME | PREFAB]`.
    pub const TAB_ORDER: [Self; 3] = [Self::Terrain, Self::Theme, Self::Prefab];

    /// The mode at top-bar tab `index`, or [`None`] if the index is out of range — the inverse of
    /// [`tab_index`](EditorMode::tab_index).
    #[must_use]
    pub fn from_tab_index(index: usize) -> Option<Self> {
        Self::TAB_ORDER.get(index).copied()
    }

    /// This mode's position in the top-bar tab order — the index the active tab highlights.
    #[must_use]
    pub fn tab_index(self) -> usize {
        Self::TAB_ORDER
            .iter()
            .position(|mode| *mode == self)
            .unwrap_or(0)
    }

    /// The human label shown on this mode's top-bar tab (and in the status line).
    #[must_use]
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Terrain => "TERRAIN",
            Self::Theme => "THEME",
            Self::Prefab => "PREFAB",
        }
    }
}

/// `Update` (in `Editing`): number-key hotkeys set the [`EditorMode`] (the `1`/`2`/`3` mode
/// hotkeys, preserved across the egui swap — GTW-512 C1.3).
///
/// `1` → [`Terrain`](EditorMode::Terrain), `2` → [`Theme`](EditorMode::Theme), `3` →
/// [`Prefab`](EditorMode::Prefab) — the `[TERRAIN | THEME | PREFAB]` tab order. Writes with
/// [`set_if_neq`](DetectChangesMut::set_if_neq) so an unchanged key-press is a no-op. Guarded on the
/// optional [`EditorMode`] (state-scoped — bevy-traps #1). UI-agnostic: the egui tabs and these keys
/// both write the same resource, and the next-frame egui draw reflects the change.
pub(crate) fn mode_hotkeys(keys: Res<ButtonInput<KeyCode>>, mode: Option<ResMut<EditorMode>>) {
    let Some(mut mode) = mode else {
        return;
    };
    let pressed = if keys.just_pressed(KeyCode::Digit1) {
        Some(EditorMode::Terrain)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(EditorMode::Theme)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(EditorMode::Prefab)
    } else {
        None
    };
    if let Some(next) = pressed {
        mode.set_if_neq(next);
    }
}
