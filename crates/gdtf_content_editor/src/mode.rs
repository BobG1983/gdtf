//! The Workbench **editor-mode** machine (GTW-474) — the [`EditorMode`] resource, the
//! top-bar mode tabs, the per-mode content-subtree markers, and the systems that drive a mode
//! switch by toggling [`Visibility`] (NEVER despawn+respawn — the ui-mutate-in-place rule).
//!
//! The editor grew from a single prefab painter into a multi-mode "Workbench" tool. The shell
//! keeps the verbatim four regions ([`regions`](crate::regions)); each region hosts ONE
//! per-mode content container per mode, all spawned once `OnEnter(Editing)`, and exactly one
//! mode's containers are [`Visible`](Visibility::Inherited) at a time. The mode is the
//! state-scoped [`EditorMode`] resource (NOT a `SubStates` — bevy-traps #5), default
//! [`Prefab`](EditorMode::Prefab) (the existing painter), driven by:
//!
//! - [`apply_mode_switch`] — a [`SegmentSelected`] from the [`EditorModeTabs`] segmented
//!   control maps the chosen index to the [`EditorMode`].
//! - [`mode_hotkeys`] — number keys (`1` = Terrain, `2` = Prefab) set the [`EditorMode`].
//! - [`toggle_mode_content`] — on a [`EditorMode`] change, flips [`Visibility`] on every
//!   per-mode content container so only the active mode's subtrees show (mutate-in-place).
//!
//! For THIS ticket the tabs are `[TERRAIN | PREFAB]`, both fully functional (the THEME mode is
//! GTW-475 — no dead tab is shipped).

use bevy::prelude::*;
use gdtf_ui::{SegmentLabel, SegmentSelected};

/// The active **editor mode** — which authoring workflow the Workbench is showing (GTW-474).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), NOT a `SubStates` (it would trip bevy-traps #5 and the `OnEnter`
/// command-flush race the palette / right panel already work around). The mode toggles
/// [`Visibility`] on pre-spawned per-mode content containers rather than despawning/respawning
/// them (the ui-mutate-in-place rule).
///
/// A named closed enum (no-bare-types: a mode is a domain value). The variant ORDER is the
/// top-bar tab order, so [`from_tab_index`](EditorMode::from_tab_index) /
/// [`tab_index`](EditorMode::tab_index) map a tab index to a mode and back. The default is
/// [`Prefab`](EditorMode::Prefab) — the existing painter the editor opened in before the
/// Workbench grew around it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorMode {
    /// TERRAIN authoring — create a terrain definition (`*.terrain_def.ron`).
    Terrain,
    /// PREFAB authoring — paint a prefab with the selected theme (the original editor).
    #[default]
    Prefab,
}

impl EditorMode {
    /// The mode-tab order, left to right — the labels the [`EditorModeTabs`] segmented control
    /// renders and the index order [`from_tab_index`](EditorMode::from_tab_index) maps. THEME is
    /// deliberately ABSENT (GTW-475) so no dead tab ships.
    pub const TAB_ORDER: [Self; 2] = [Self::Terrain, Self::Prefab];

    /// The mode at top-bar tab `index`, or [`None`] if the index is out of range — the inverse
    /// of [`tab_index`](EditorMode::tab_index). Used by [`apply_mode_switch`] to map a
    /// [`SegmentSelected`] index to a mode.
    #[must_use]
    pub fn from_tab_index(index: usize) -> Option<Self> {
        Self::TAB_ORDER.get(index).copied()
    }

    /// This mode's position in the top-bar tab order — the index the [`EditorModeTabs`] control
    /// pre-selects / highlights for the active mode.
    #[must_use]
    pub fn tab_index(self) -> usize {
        Self::TAB_ORDER
            .iter()
            .position(|mode| *mode == self)
            .unwrap_or(0)
    }

    /// The human label shown on this mode's top-bar tab.
    #[must_use]
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Terrain => "TERRAIN",
            Self::Prefab => "PREFAB",
        }
    }

    /// The top-bar mode-tab segment labels, in tab order — the `labels` slice
    /// [`spawn_segmented_control`](gdtf_ui::spawn_segmented_control) renders.
    #[must_use]
    pub fn tab_labels() -> Vec<SegmentLabel> {
        Self::TAB_ORDER
            .iter()
            .map(|mode| SegmentLabel::new(mode.tab_label()))
            .collect()
    }
}

/// Identity marker on the top-bar mode-tabs [`SegmentedControl`](gdtf_ui::SegmentedControl)
/// root, so [`apply_mode_switch`] can filter a [`SegmentSelected`] to THIS control (a stray
/// segmented control elsewhere never drives the mode).
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct EditorModeTabs;

/// Marker on a TERRAIN-mode content container — a subtree shown only while
/// [`EditorMode::Terrain`] is active (GTW-474). [`toggle_mode_content`] sets its
/// [`Visibility`].
///
/// A unit marker — presence alone is the signal (no-bare-types rule). One per region the
/// TERRAIN form fills (the regions stay verbatim; the per-mode container is the new subtree
/// root the mockup specifies).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TerrainModeContent;

/// Marker on a PREFAB-mode content container — a subtree shown only while
/// [`EditorMode::Prefab`] is active (GTW-474). [`toggle_mode_content`] sets its
/// [`Visibility`].
///
/// A unit marker — presence alone is the signal (no-bare-types rule). The existing painter's
/// palette / canvas / right-panel / stat content parents into these containers, so toggling the
/// container's visibility hides/shows the whole prefab UI at once (mutate-in-place, never
/// despawn).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PrefabModeContent;

/// `Update` (in `Editing`): map a top-bar mode-tab selection to the [`EditorMode`] (C1).
///
/// Reads [`SegmentSelected`] filtered to THIS editor's [`EditorModeTabs`] control, maps the
/// chosen segment index to a mode via [`EditorMode::from_tab_index`], and writes it with
/// [`set_if_neq`](DetectChangesMut::set_if_neq) so [`toggle_mode_content`] reacts only on a real
/// change. Guarded on the optional [`EditorMode`] (state-scoped — bevy-traps #1).
pub(crate) fn apply_mode_switch(
    mut selections: MessageReader<SegmentSelected>,
    tabs: Query<(), With<EditorModeTabs>>,
    mode: Option<ResMut<EditorMode>>,
) {
    let Some(mut mode) = mode else {
        return;
    };
    for selection in selections.read() {
        if tabs.get(selection.control).is_err() {
            continue;
        }
        if let Some(next) = EditorMode::from_tab_index(*selection.index) {
            mode.set_if_neq(next);
        }
    }
}

/// `Update` (in `Editing`): number-key hotkeys set the [`EditorMode`] (C1).
///
/// `1` → [`Terrain`](EditorMode::Terrain), `2` → [`Prefab`](EditorMode::Prefab) — the tab order.
/// Writes with [`set_if_neq`](DetectChangesMut::set_if_neq) so an unchanged key-press neither
/// re-toggles nor re-syncs the tabs. Guarded on the optional [`EditorMode`] (bevy-traps #1).
pub(crate) fn mode_hotkeys(keys: Res<ButtonInput<KeyCode>>, mode: Option<ResMut<EditorMode>>) {
    let Some(mut mode) = mode else {
        return;
    };
    let pressed = if keys.just_pressed(KeyCode::Digit1) {
        Some(EditorMode::Terrain)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(EditorMode::Prefab)
    } else {
        None
    };
    if let Some(next) = pressed {
        mode.set_if_neq(next);
    }
}

/// `Update` (in `Editing`): show the active mode's content containers and hide the others —
/// MUTATING [`Visibility`] in place, never despawning (C1, the ui-mutate-in-place rule).
///
/// Runs whenever the [`EditorMode`] changed this frame (a `Changed`-style `is_changed()` gate
/// covers the first insert too, so the initial mode is applied once). Sets every
/// [`TerrainModeContent`] container to [`Visible`](Visibility::Inherited) /
/// [`Hidden`](Visibility::Hidden) by whether [`Terrain`](EditorMode::Terrain) is active, and the
/// [`PrefabModeContent`] containers by the inverse, with [`set_if_neq`] hygiene. Visibility
/// INHERITS to each container's children, so content spawned into a container later (e.g. the
/// palette rows a theme switch rebuilds) tracks the container's visibility automatically.
pub(crate) fn toggle_mode_content(
    mode: Option<Res<EditorMode>>,
    mut terrain: Query<&mut Visibility, (With<TerrainModeContent>, Without<PrefabModeContent>)>,
    mut prefab: Query<&mut Visibility, (With<PrefabModeContent>, Without<TerrainModeContent>)>,
) {
    let Some(mode) = mode else {
        return;
    };
    if !mode.is_changed() {
        return;
    }
    let terrain_active = matches!(*mode, EditorMode::Terrain);
    let (terrain_vis, prefab_vis) = if terrain_active {
        (Visibility::Inherited, Visibility::Hidden)
    } else {
        (Visibility::Hidden, Visibility::Inherited)
    };
    for mut vis in &mut terrain {
        vis.set_if_neq(terrain_vis);
    }
    for mut vis in &mut prefab {
        vis.set_if_neq(prefab_vis);
    }
}

/// `Update` (in `Editing`): keep the top-bar mode-tabs' active segment in sync with the
/// [`EditorMode`] when a HOTKEY (not a tab click) changed it (C1).
///
/// A hotkey sets the [`EditorMode`] directly without touching the segmented control's
/// [`ActiveSegment`](gdtf_ui::ActiveSegment); this writes the mode's
/// [`tab_index`](EditorMode::tab_index) onto the [`EditorModeTabs`] control so the highlighted
/// tab follows. `gdtf_ui`'s `repaint_segments` then repaints. Guarded + change-gated +
/// [`set_if_neq`] so a tab-click (which already moved the segment) does not double-write.
pub(crate) fn sync_tabs_to_mode(
    mode: Option<Res<EditorMode>>,
    mut tabs: Query<&mut gdtf_ui::ActiveSegment, With<EditorModeTabs>>,
) {
    let Some(mode) = mode else {
        return;
    };
    if !mode.is_changed() {
        return;
    }
    let want = gdtf_ui::ActiveSegment::new(mode.tab_index());
    for mut active in &mut tabs {
        active.set_if_neq(want);
    }
}
