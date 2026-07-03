//! The PREFAB-mode **controls** (GTW-515 C4.5 / C4.6 / C4.9) — the RIGHT panel: the grid SIZE
//! fields, the LEVEL navigation (chrome buttons + readout; the `]`/`[`/PageUp/PageDown hotkeys live
//! in a `Update` system), and the debug-only Save-prefab control.
//!
//! ## Size fields (C4.6 / GTW-464)
//!
//! The W / H / levels size fields moved to the sibling [`size_fields`](super::size_fields) module
//! (GTW-464), which models the displayed spans explicitly
//! ([`SizeFieldSpans`](super::size_fields::SizeFieldSpans)): derived FRESH from the session every
//! pass (the session → fields reverse sync) and committed back through the kept clamp on a real
//! edit. This panel just draws them ([`size_fields::size_fields`](super::size_fields::size_fields)).
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
use gdtf_battle_sim::level::UuidThemeRegistry;

use crate::{
    canvas::{CurrentEditLevel, LevelStep},
    egui_shell::prefab::size_fields,
    session::MapEditorSession,
};

/// Draw the PREFAB-mode controls into the RIGHT panel (GTW-515 C4.5 / C4.6 / C4.9; GTW-532 the
/// view toggle).
///
/// The size fields display + commit through the [`size_fields`] view model (session → fields
/// reverse sync + the kept clamp commit — GTW-464) and re-clamp the edit level on a size change;
/// the level-nav buttons step the [`CurrentEditLevel`]; the view toggle
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

    size_fields::size_fields(ui, session, edit_level);
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
/// [`FullView`](ViewMode::FullView) (draw the whole storey stack) via the presenter-owned
/// [`ViewMode::toggled`] (GTW-577 C8 — the ONE flip, no local copy). Set-to-target (an explicit
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
            *view = view.toggled();
        }
    });
    ui.label(match *view {
        ViewMode::DownToActive => "Drawing storeys 0..=active (F to toggle)",
        ViewMode::FullView => "Drawing ALL storeys (F to toggle)",
    });
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
