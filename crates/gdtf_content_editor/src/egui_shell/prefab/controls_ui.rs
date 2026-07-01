//! The PREFAB-mode **controls** (GTW-515 C4.5 / C4.6 / C4.9) — the RIGHT panel: the grid SIZE
//! fields, the LEVEL navigation (chrome buttons + readout; the `]`/`[`/PageUp/PageDown hotkeys live
//! in a `Update` system), and the debug-only Save-prefab control.
//!
//! ## Size fields (C4.6)
//!
//! W / H / levels [`egui::DragValue`]s clamped to the sim's fixed system-constant bands
//! (`1..=`[`MAX_GRID_SPAN`] on x/y, `1..=`[`MAX_LEVELS`] on z) via [`clamp_to_grid_size`], driving
//! [`GridSize::new`] / [`MapEditorSession::set_grid_size`]. No bare numeric — the value flows
//! through the [`GridWidth`] / [`GridHeight`] / [`GridLevels`] newtypes into the validated
//! [`GridSize`]. A size change re-clamps the current edit level so it never points past a shrunk
//! volume (the kept level-nav clamp — [`CurrentEditLevel::clamped`]).
//!
//! ## Level nav (C4.5)
//!
//! A `Level n / m` readout + `[▲]` / `[▼]` buttons that step the [`CurrentEditLevel`] within
//! `[1, levels]` (the kept clamp — [`CurrentEditLevel::stepped`]). The same clamp the hotkeys use.
//!
//! ## View toggle (GTW-532)
//!
//! A `[Down-to-active | Full view]` toggle button that flips the prefab viewport's [`ViewMode`]
//! (REUSED from the presenter — the SAME type the GTW-521 battlescape toggle drives). In
//! [`ViewMode::DownToActive`] the viewport draws `0..=CurrentEditLevel`; in [`ViewMode::FullView`]
//! it draws the whole prefab storey stack at once. The `F` hotkey ([`view_mode_hotkey`](super::nav::view_mode_hotkey)) flips the
//! same resource, so button + key agree (mirroring the mode / level-nav dual controls).
//!
//! ## Save (C4.9)
//!
//! A `#[cfg(debug_assertions)]` "Save prefab" button + name field → [`write_prefab`] (reused). In a
//! release build the save controls do not compile in (the whole save path is debug-only), and the
//! function signature is uniform across profiles (the release arm consumes the save-only bindings).

use bevy_egui::egui;
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, MAX_GRID_SPAN, UuidThemeRegistry,
};

use crate::{
    canvas::{CurrentEditLevel, LevelStep},
    session::MapEditorSession,
};

/// The inclusive width/height [`DragValue`](egui::DragValue) range — the sim's fixed
/// `1..=`[`MAX_GRID_SPAN`] system
/// constant (the ticket fixes this bound, so it is a system constant, not a tunable).
const SPAN_RANGE: core::ops::RangeInclusive<u8> = 1..=MAX_GRID_SPAN;

/// The inclusive levels [`DragValue`](egui::DragValue) range — the sim's fixed `1..=`[`MAX_LEVELS`]
/// system constant.
const LEVELS_RANGE: core::ops::RangeInclusive<u8> = 1..=gdtf_battle_sim::metric::MAX_LEVELS;

/// Clamp three raw axis spans into a validated [`GridSize`] (GTW-515 C4.6) — the PURE size-field
/// clamp the unit test exercises on the real path (C4.13).
///
/// Each axis is clamped to its fixed system-constant band (`1..=`[`MAX_GRID_SPAN`] on x/y,
/// `1..=`[`MAX_LEVELS`] on z), wrapped in its [`GridWidth`] / [`GridHeight`] / [`GridLevels`]
/// newtype (no bare numeric), and validated through [`GridSize::new`]. Because every axis is
/// pre-clamped into range, `GridSize::new` never rejects — but a defensive fall-back to `previous`
/// keeps the function total + panic-free if the sim's bounds ever tighten below the clamp.
#[must_use]
pub(crate) fn clamp_to_grid_size(w: u8, h: u8, levels: u8, previous: GridSize) -> GridSize {
    let width = GridWidth::new(w.clamp(*SPAN_RANGE.start(), *SPAN_RANGE.end()));
    let height = GridHeight::new(h.clamp(*SPAN_RANGE.start(), *SPAN_RANGE.end()));
    let levels = GridLevels::new(levels.clamp(*LEVELS_RANGE.start(), *LEVELS_RANGE.end()));
    GridSize::new(width, height, levels).unwrap_or(previous)
}

/// Draw the PREFAB-mode controls into the RIGHT panel (GTW-515 C4.5 / C4.6 / C4.9; GTW-532 the
/// view toggle).
///
/// The size fields commit through [`clamp_to_grid_size`] → [`MapEditorSession::set_grid_size`] and
/// re-clamp the edit level; the level-nav buttons step the [`CurrentEditLevel`]; the view toggle
/// flips the [`ViewMode`]; the debug Save control writes the prefab. `save_name` is the in-UI
/// prefab-name buffer the shell owns across frames (an [`egui::TextEdit`] needs a persistent
/// `&mut String`); `themes` resolves the active theme's display name for the save path.
#[expect(
    clippy::too_many_arguments,
    reason = "the controls panel drives every prefab RIGHT-panel control from the borrows the shell \
              already holds (session + edit level + view mode + the debug save name/map/registries + \
              themes); each is a distinct borrow threaded through the shell's panel closure and \
              Bevy's injection cannot reduce them without a wrapper that changes the crate API"
)]
pub(crate) fn controls_panel(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    edit_level: &mut CurrentEditLevel,
    view: &mut ViewMode,
    #[cfg_attr(
        not(debug_assertions),
        expect(unused_variables, reason = "save-only in debug")
    )]
    save_name: &mut String,
    #[cfg_attr(
        not(debug_assertions),
        expect(unused_variables, reason = "save-only in debug")
    )]
    map: &crate::editor_map::EditorMap,
    #[cfg_attr(
        not(debug_assertions),
        expect(unused_variables, reason = "save-only in debug")
    )]
    registry: Option<&gdtf_battle_sim::terrain::def::TerrainDefRegistry>,
    themes: Option<&UuidThemeRegistry>,
) {
    ui.heading("Prefab");
    ui.separator();

    size_fields(ui, session, edit_level);
    ui.separator();
    level_nav(ui, session, edit_level);
    ui.separator();
    view_toggle(ui, view);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_control(ui, session, map, registry, themes, save_name);
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = themes;
    }
}

/// The prefab-viewport VIEW toggle (GTW-532 C3) — a labelled button that flips the [`ViewMode`]
/// between [`DownToActive`](ViewMode::DownToActive) (draw `0..=CurrentEditLevel`) and
/// [`FullView`](ViewMode::FullView) (draw the whole storey stack). Set-to-target (an explicit
/// assignment to the toggled mode), so it is idempotent under the egui multipass re-run
/// (bevy-traps #8 (b)). The `F` hotkey ([`view_mode_hotkey`](super::nav::view_mode_hotkey)) flips the SAME resource.
fn view_toggle(ui: &mut egui::Ui, view: &mut ViewMode) {
    ui.label("Storey view");
    ui.horizontal(|ui| {
        // The button LABEL names the current mode; clicking it flips to the other (the standard
        // two-state toggle affordance).
        let label = match *view {
            ViewMode::DownToActive => "Down-to-active ▸ Full view",
            ViewMode::FullView => "Full view ▸ Down-to-active",
        };
        if ui.button(label).clicked() {
            *view = toggled(*view);
        }
    });
    ui.label(match *view {
        ViewMode::DownToActive => "Drawing storeys 0..=active (F to toggle)",
        ViewMode::FullView => "Drawing ALL storeys (F to toggle)",
    });
}

/// The OTHER [`ViewMode`] — the pure flip the toggle button and the `F` hotkey both apply. Pure so
/// the toggle is unit-tested on the real path (GTW-532 C4).
#[must_use]
pub(crate) const fn toggled(view: ViewMode) -> ViewMode {
    match view {
        ViewMode::DownToActive => ViewMode::FullView,
        ViewMode::FullView => ViewMode::DownToActive,
    }
}

/// The W / H / levels size fields (C4.6) — three [`egui::DragValue`]s clamped to the sim bands,
/// committed through [`clamp_to_grid_size`] into the session's [`GridSize`], with the edit level
/// re-clamped on change (the kept clamp).
fn size_fields(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    edit_level: &mut CurrentEditLevel,
) {
    let size = session.grid_size();
    let mut w = *size.width();
    let mut h = *size.height();
    let mut levels = *size.levels();
    let mut changed = false;

    ui.label("Grid size (cells)");
    ui.horizontal(|ui| {
        ui.label("W");
        changed |= ui
            .add(egui::DragValue::new(&mut w).range(SPAN_RANGE))
            .changed();
        ui.label("H");
        changed |= ui
            .add(egui::DragValue::new(&mut h).range(SPAN_RANGE))
            .changed();
        ui.label("Levels");
        changed |= ui
            .add(egui::DragValue::new(&mut levels).range(LEVELS_RANGE))
            .changed();
    });

    if changed {
        let new_size = clamp_to_grid_size(w, h, levels, size);
        session.set_grid_size(new_size);
        // Re-clamp the edit level so a shrunk volume never leaves it pointing past the new extent.
        *edit_level = edit_level.clamped(new_size);
    }
}

/// The level-nav readout + step buttons (C4.5) — a `Level n / m` readout (1-based for the reader)
/// and `[▲]` / `[▼]` buttons stepping the [`CurrentEditLevel`] within `[1, levels]` (the kept
/// clamp — [`CurrentEditLevel::stepped`], the SAME clamp the hotkeys use).
fn level_nav(ui: &mut egui::Ui, session: &MapEditorSession, edit_level: &mut CurrentEditLevel) {
    let size = session.grid_size();
    let levels = *size.levels();
    // The stored Level is a 0-based storey index; show it 1-based for the reader.
    let current_1based = u16::from(*edit_level.level()).saturating_add(1);
    ui.horizontal(|ui| {
        ui.label(format!("Level {current_1based} / {levels}"));
        if ui.button("▼").clicked() {
            *edit_level = edit_level.stepped(LevelStep::down(), size);
        }
        if ui.button("▲").clicked() {
            *edit_level = edit_level.stepped(LevelStep::up(), size);
        }
    });
}

/// The debug-only Save-prefab control (C4.9) — a name field + a "Save prefab" button; on press it
/// resolves the active theme's display name and calls the reused
/// [`write_prefab`](crate::save::write_prefab). On a typed error it logs and writes nothing (never a
/// panic). Debug-only — the whole prefab save path is `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_control(
    ui: &mut egui::Ui,
    session: &MapEditorSession,
    map: &crate::editor_map::EditorMap,
    registry: Option<&gdtf_battle_sim::terrain::def::TerrainDefRegistry>,
    themes: Option<&UuidThemeRegistry>,
    save_name: &mut String,
) {
    ui.label("Prefab name");
    ui.text_edit_singleline(save_name);
    if !ui.button("Save prefab").clicked() {
        return;
    }
    let Some(registry) = registry else {
        bevy::log::error!("prefab save: terrain registry not resolved");
        return;
    };
    let theme_display = themes
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone());
    match crate::save::write_prefab(map, registry, session, &theme_display, save_name) {
        Ok(path) => bevy::log::info!("prefab save: wrote prefab to `{}`", path.display()),
        Err(err) => bevy::log::error!("prefab save: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::level::{GridSize, MAX_GRID_SPAN};

    use super::clamp_to_grid_size;

    /// C4.13 — the size-field clamp folds an over-max width/height to `MAX_GRID_SPAN` and an
    /// over-max level count to `MAX_LEVELS` (the ticket fixes these bounds, so pinning THEM is
    /// allowed — C4.13). A zero span clamps UP to 1 (never the empty grid `GridSize::new` rejects).
    #[test]
    fn clamp_folds_into_the_system_constant_bands() {
        let previous = GridSize::default();
        // Over-max on every axis → each axis saturates at its ceiling.
        let clamped = clamp_to_grid_size(200, 200, 99, previous);
        assert_eq!(
            *clamped.width(),
            MAX_GRID_SPAN,
            "width clamps to MAX_GRID_SPAN (60)"
        );
        assert_eq!(
            *clamped.height(),
            MAX_GRID_SPAN,
            "height clamps to MAX_GRID_SPAN (60)"
        );
        assert_eq!(
            *clamped.levels(),
            gdtf_battle_sim::metric::MAX_LEVELS,
            "levels clamps to MAX_LEVELS (8)",
        );
        // Zero span → clamps UP to 1 on every axis (a valid 1x1x1, not the rejected empty grid).
        let floored = clamp_to_grid_size(0, 0, 0, previous);
        assert_eq!(*floored.width(), 1, "zero width clamps up to 1");
        assert_eq!(*floored.height(), 1, "zero height clamps up to 1");
        assert_eq!(*floored.levels(), 1, "zero levels clamps up to 1");
    }

    /// C4.13 — an in-range size passes through unchanged (the clamp only bites at the bounds).
    #[test]
    fn clamp_passes_in_range_sizes_through() {
        let previous = GridSize::default();
        let clamped = clamp_to_grid_size(16, 24, 3, previous);
        assert_eq!(*clamped.width(), 16);
        assert_eq!(*clamped.height(), 24);
        assert_eq!(*clamped.levels(), 3);
    }
}
