//! The THEME-mode form's **drive systems** (GTW-475): capture the display-name commit, toggle the
//! terrain multi-select, (re)build the terrain-library rows, keep the default-floor dropdown's
//! options in sync with the selected palette (Slab-preferred — C6), capture a default-floor pick,
//! load an existing theme on a top-bar dropdown change (C4), and reset the form on the New-theme
//! press (C4).
//!
//! The read-only RENDER systems (the C3 resolved-stats readout + HP bar, the read-only KEY text,
//! the live `.terrain_theme.ron` preview) and the debug-only SAVE trigger live in
//! [`render`](super::render). Every system is param-only (no `&mut World` — bevy-traps #7) and
//! guards the state-scoped [`ThemeDraft`] with `Option<Res<…>>` (bevy-traps #1).

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainUuid},
};
use gdtf_ui::{
    ActiveButton, DropdownLabel, DropdownOption, DropdownOptions, DropdownSelectionChanged,
    SelectedIndex, TextFieldCommitted, theme::GdtfTheme,
};

use super::{
    spawn::{sim_kind_label, spawn_terrain_library_row},
    types::{NewThemeButton, ThemeDefaultFloorPicker, ThemeDraft, ThemeNameField, ThemeTerrainRow},
};
use crate::{mode::EditorMode, right_panel::ThemeDropdown};

/// `Update` (in `Editing`): commit the display-name text field into the draft (C2).
pub(crate) fn commit_theme_name(
    mut commits: MessageReader<TextFieldCommitted>,
    fields: Query<(), With<ThemeNameField>>,
    draft: Option<ResMut<ThemeDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for commit in commits.read() {
        if fields.get(commit.field()).is_err() {
            continue;
        }
        draft.set_display_name(commit.value().value().to_owned());
    }
}

/// The press-edge query filter for a terrain-library row — a row whose [`Interaction`] changed
/// this frame. A named alias to keep the system signature under clippy's `type_complexity` gate.
type PressedLibraryRow = (Changed<Interaction>, With<ThemeTerrainRow>);

/// `Update` (in `Editing`): a clicked terrain-library row flips that terrain in the draft's
/// multi-select palette + repaints its highlight (C2).
pub(crate) fn toggle_theme_terrain(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction, &ThemeTerrainRow), PressedLibraryRow>,
    draft: Option<ResMut<ThemeDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for (entity, interaction, row) in &pressed {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        draft.toggle_terrain(row.terrain());
        if draft.has_terrain(row.terrain()) {
            commands.entity(entity).insert(ActiveButton);
        } else {
            commands.entity(entity).remove::<ActiveButton>();
        }
    }
}

/// `Update` (in `Editing`): (re)build the LEFT terrain-library rows from the
/// [`TerrainDefRegistry`] — the initial populate AND a rebuild when the loaded library changes
/// (a hot-reload adds a def), in ONE system (the GTW-422 palette-sync precedent).
///
/// The registry is inserted via a deferred command on `OnEnter`, so an `OnEnter` populate would
/// race it — this runs in `Update` and (re)builds when the registry's def count differs from the
/// count the rows were last built for. Despawns the existing [`ThemeTerrainRow`] entities first,
/// then re-lists every def (sorted by display name for determinism), parenting them under the
/// LEFT region's THEME container. A row carries its [`TerrainUuid`] for the toggle handler and is
/// pre-highlighted if it is already in the draft palette (so a loaded theme — C4 — shows its
/// members selected).
pub(crate) fn sync_theme_library(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    terrain: Option<Res<TerrainDefRegistry>>,
    draft: Option<Res<ThemeDraft>>,
    rows: Query<Entity, With<ThemeTerrainRow>>,
    mut last_count: Local<Option<usize>>,
    mut built_for: Local<Option<ThemeUuid>>,
) {
    let (Some(terrain), Some(draft)) = (terrain, draft) else {
        return;
    };
    let count = terrain.len();
    // Rebuild when the loaded library size changes OR the draft key changes (a New-theme reset /
    // a load swaps which rows are pre-highlighted — C4).
    if *last_count == Some(count) && *built_for == Some(draft.key()) {
        return;
    }
    for row in &rows {
        commands.entity(row).despawn();
    }
    let row_bg = *theme.panel.color;
    let text_color = *theme.text.text_color;
    let mut entries: Vec<(String, TerrainUuid)> = terrain
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let new_rows: Vec<Entity> = entries
        .iter()
        .filter_map(|(_, key)| terrain.def(key).map(|def| (key, def)))
        .map(|(key, def)| {
            spawn_terrain_library_row(
                &mut commands,
                *key,
                def,
                draft.has_terrain(*key),
                row_bg,
                text_color,
            )
        })
        .collect();
    parent_rows_under_left(&mut commands, new_rows);
    *last_count = Some(count);
    *built_for = Some(draft.key());
}

/// Parent the freshly-built library rows under the LEFT region's THEME-mode content container (the
/// deferred-command parenting idiom — the `terrain_form` / palette precedent).
fn parent_rows_under_left(commands: &mut Commands, rows: Vec<Entity>) {
    commands.queue(move |world: &mut World| {
        let host = crate::mode_host::mode_host_under_region::<
            crate::LeftPaletteRegion,
            crate::mode::ThemeModeContent,
        >(world);
        let Some(host) = host else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            for row in rows {
                host_entity.add_child(row);
            }
        }
    });
}

/// `Update` (in `Editing`): keep the DEFAULT-FLOOR dropdown's options in sync with the draft's
/// selected terrain palette (C6) — Slab-kind FIRST (a Slab is the walkable floor; `Floor` was
/// retired in GTW-476), then the rest, each labeled by the def display name + sim kind.
///
/// Runs when the draft CHANGED (covers every multi-select toggle + a load / reset). MUTATES the
/// dropdown's [`DropdownOptions`] / [`SelectedIndex`] components + the shown [`DropdownLabel`]
/// child text IN PLACE (never despawn — the ui-mutate rule; the dropdown has no public option-
/// mutation API, so we re-insert the data components and rewrite the label child directly). The
/// pre-selected index tracks the draft's current default floor, or the first option (the
/// defensible default per C6: the first selected terrain) when none is chosen yet.
pub(crate) fn sync_default_floor_options(
    draft: Option<Res<ThemeDraft>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    controls: Query<(Entity, &Children), With<ThemeDefaultFloorPicker>>,
    mut options: Query<&mut DropdownOptions<TerrainUuid>>,
    mut selected: Query<&mut SelectedIndex>,
    mut labels: Query<&mut Text, With<DropdownLabel>>,
    mut commands: Commands,
) {
    let (Some(draft), Some(terrain)) = (draft, terrain) else {
        return;
    };
    if !draft.is_changed() {
        return;
    }
    // Slab-kind terrain FIRST (walkable-floor preferred — C6), then the rest, both in the draft's
    // selection order so the list is stable.
    let candidates = floor_candidates(&draft, &terrain);
    let new_options: Vec<DropdownOption<TerrainUuid>> = candidates
        .iter()
        .map(|(key, label)| DropdownOption::new(*key, label.clone()))
        .collect();
    // The pre-selected index: the draft's chosen floor if present, else 0 (the first candidate —
    // the defensible default per C6).
    let select_idx = draft
        .default_floor()
        .and_then(|floor| candidates.iter().position(|(key, _)| *key == floor))
        .unwrap_or(0);
    let shown = candidates
        .get(select_idx)
        .map_or_else(String::new, |(_, label)| label.clone());

    for (control, children) in &controls {
        if let Ok(mut opts) = options.get_mut(control) {
            *opts = DropdownOptions::new(new_options.clone());
        } else {
            commands
                .entity(control)
                .insert(DropdownOptions::new(new_options.clone()));
        }
        if let Ok(mut idx) = selected.get_mut(control) {
            *idx = SelectedIndex::new(select_idx);
        }
        for &child in children {
            if let Ok(mut text) = labels.get_mut(child) {
                *text = Text::new(shown.clone());
            }
        }
    }
}

/// The default-floor candidate list for a draft: its selected terrain, Slab-kind FIRST (C6), each
/// as `(key, "display name [Kind]")`. Pure, so the C7 test pins the Slab-preference ordering.
#[must_use]
pub(crate) fn floor_candidates(
    draft: &ThemeDraft,
    terrain: &TerrainDefRegistry,
) -> Vec<(TerrainUuid, String)> {
    let mut slabs: Vec<(TerrainUuid, String)> = Vec::new();
    let mut others: Vec<(TerrainUuid, String)> = Vec::new();
    for key in draft.terrain() {
        let Some(def) = terrain.def(key) else {
            continue;
        };
        let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
        if matches!(def.sim_kind, TerrainSimKind::Slab { .. }) {
            slabs.push((*key, label));
        } else {
            others.push((*key, label));
        }
    }
    slabs.extend(others);
    slabs
}

/// `Update` (in `Editing`): a default-floor-dropdown selection sets the draft default floor
/// (C6 — the draft setter ignores it unless the terrain is in the selected palette, fail-closed).
pub(crate) fn apply_default_floor(
    mut changes: MessageReader<DropdownSelectionChanged<TerrainUuid>>,
    pickers: Query<(), With<ThemeDefaultFloorPicker>>,
    draft: Option<ResMut<ThemeDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for change in changes.read() {
        if pickers.get(change.control()).is_err() {
            continue;
        }
        draft.set_default_floor(*change.id());
    }
}

/// `Update` (in `Editing`): while in THEME mode, a top-bar GLOBAL theme dropdown change LOADS that
/// theme into the form for editing (C4).
///
/// Reads [`DropdownSelectionChanged<ThemeUuid>`] from the top-bar [`ThemeDropdown`], resolves the
/// [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) from the [`UuidThemeRegistry`], and
/// replaces the draft with that theme's parts (display name / terrain palette / default floor) so
/// the form populates for editing. Gated to THEME mode (the PREFAB / TERRAIN modes use the global
/// dropdown for their own purposes — it must not clobber the theme draft outside THEME mode).
pub(crate) fn load_theme_into_form(
    mut changes: MessageReader<DropdownSelectionChanged<ThemeUuid>>,
    dropdowns: Query<(), With<ThemeDropdown>>,
    mode: Option<Res<EditorMode>>,
    themes: Option<Res<UuidThemeRegistry>>,
    draft: Option<ResMut<ThemeDraft>>,
) {
    let (Some(mode), Some(themes), Some(mut draft)) = (mode, themes, draft) else {
        // Drain so a non-THEME-mode change does not stick in the buffer for the next frame.
        changes.clear();
        return;
    };
    if !matches!(*mode, EditorMode::Theme) {
        changes.clear();
        return;
    }
    for change in changes.read() {
        if dropdowns.get(change.control()).is_err() {
            continue;
        }
        let key = *change.id();
        if let Some(def) = themes.def(&key) {
            *draft = ThemeDraft::from_parts(
                def.key,
                (*def.display_name).clone(),
                def.terrain.clone(),
                def.default_floor,
            );
        }
    }
}

/// The press-edge query filter for the New-theme button — a button whose [`Interaction`] changed
/// this frame. A named alias to keep the system signature under clippy's `type_complexity` gate.
type PressedNewButton = (Changed<Interaction>, With<NewThemeButton>);

/// `Update` (in `Editing`): the New-theme button mints a fresh [`ThemeUuid`] + CLEARS the form
/// (C4) — replaces the draft with a brand-new [`ThemeDraft::new_theme`].
pub(crate) fn reset_theme_form_on_new(
    buttons: Query<&Interaction, PressedNewButton>,
    draft: Option<ResMut<ThemeDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    if buttons
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Pressed))
    {
        *draft = ThemeDraft::new_theme();
    }
}

// The read-only render systems — the read-only KEY text, the C3 resolved-stats readout + HP bar,
// and the live `.terrain_theme.ron` preview — plus the debug-only Save-theme trigger live in
// `super::render` (split out to keep each file within the size caps).
