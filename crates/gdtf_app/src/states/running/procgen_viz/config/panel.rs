//! The DEV-ONLY procgen-visualizer INPUT PANEL builder (GTW-498) — the theme dropdown, the
//! three grid-axis numeric fields, the seed numeric field, the player / enemy gang dropdowns,
//! the size-validity status text, and the Generate button.
//!
//! Spawned by [`spawn_config_panel`] as a left-anchored themed panel (so it never occludes the
//! centered board, `bevy-traps.md` #8) `OnEnter(DebugProcgenVisualizer)`, AFTER the model +
//! config are inserted. Selecting an input MUTATES [`VizConfig`](super::resource::VizConfig)
//! (the selection / commit listeners in [`super::apply`]); the Generate button RE-RUNS procgen
//! from it (C5). Every node carries `DespawnOnExit(DebugProcgenVisualizer)`. The whole module
//! is `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::{prelude::*, ui::Val};
use gdtf_battle_sim::{
    GangName, GangRegistry, ThemeUuid, UuidThemeRegistry, level::MAX_GRID_SPAN, metric::MAX_LEVELS,
};
use gdtf_ui::{
    ButtonLabel, DropdownColors, DropdownOption, FieldColors, NumericRange, spawn_button,
    spawn_dropdown, spawn_numeric_field, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::procgen_viz::config::{
        components::{
            EnemyGangDropdown, GenerateButton, HeightField, LevelsField, PlayerGangDropdown,
            SeedField, SizeStatusText, ThemeDropdown, WidthField,
        },
        resource::VizConfig,
    },
};

/// The `GlobalZIndex` of the input panel — at the control-bar band so it sits above the board
/// quads; its dropdown popups carry their own higher z (`DROPDOWN_POPUP_Z`), so they draw over
/// it (`bevy-traps.md` #8).
const PANEL_Z: i32 = 20;
/// The input panel's left inset, as a viewport-width percentage (relative units, the
/// responsive-UI rule — not px).
const PANEL_LEFT_VW: f32 = 2.0;
/// The input panel's top inset, as a viewport-height percentage.
const PANEL_TOP_VH: f32 = 10.0;
/// The input panel's width, as a viewport-width percentage.
const PANEL_WIDTH_VW: f32 = 22.0;
/// Inter-row gap inside the panel, in `Vh` (the menu's calibrated separation).
const ROW_GAP_VH: f32 = 1.388_89;

/// Spawn the configurable INPUT PANEL (C1–C5) and return its root [`Entity`] so the caller
/// parents it under the screen root.
///
/// Reads the just-inserted [`VizConfig`] for the initial selections, the
/// [`UuidThemeRegistry`] for the theme options (C1), and the [`GangRegistry`] for the gang
/// options (C4) — all `Option`, so an absent registry yields an EMPTY option list (degraded,
/// never panics; the no-gang fallback, C4). Builds a left-anchored themed column of labelled
/// rows.
pub(in crate::states::running::procgen_viz) fn spawn_config_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    config: &VizConfig,
    themes: Option<&UuidThemeRegistry>,
    gangs: Option<&GangRegistry>,
) -> Entity {
    let panel = spawn_panel(commands, theme);
    commands.entity(panel).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(PANEL_LEFT_VW),
            top: Val::Vh(PANEL_TOP_VH),
            width: Val::Vw(PANEL_WIDTH_VW),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(ROW_GAP_VH),
            ..default()
        },
        GlobalZIndex(PANEL_Z),
        DespawnOnExit(RunningState::DebugProcgenVisualizer),
    ));

    // Build each labelled row group; collect every spawned child in display order, then parent
    // them all under the panel in one call (the rows are split into focused helpers so this
    // assembler stays short).
    let mut rows = vec![label_node(commands, "INPUTS", ThemeRole::Title)];
    spawn_theme_row(commands, theme, config, themes, &mut rows);
    spawn_size_rows(commands, theme, config, &mut rows);
    spawn_seed_row(commands, theme, config, &mut rows);
    spawn_gang_rows(commands, theme, config, gangs, &mut rows);

    // SIZE-validity status text (C2) — initialised from the current combo's validity.
    rows.push(
        commands
            .spawn((
                SizeStatusText,
                Themed::new(ThemeRole::Text),
                Text::new(size_status_text(config)),
                DespawnOnExit(RunningState::DebugProcgenVisualizer),
            ))
            .id(),
    );

    // GENERATE button (C5).
    rows.push(spawn_button(
        commands,
        theme,
        ButtonLabel::new("Generate"),
        (
            GenerateButton,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));

    commands.entity(panel).add_children(&rows);
    panel
}

/// Spawn the THEME label + dropdown (C1) — populated from the UUID theme registry, labelled by
/// display name, opened on the config's current theme — and push them onto `rows`.
fn spawn_theme_row(
    commands: &mut Commands,
    theme: &GdtfTheme,
    config: &VizConfig,
    themes: Option<&UuidThemeRegistry>,
    rows: &mut Vec<Entity>,
) {
    rows.push(label_node(commands, "Theme", ThemeRole::Text));
    let options = theme_options(themes);
    let initial = option_index_of(&options, &config.theme());
    rows.push(spawn_dropdown(
        commands,
        options,
        initial,
        dropdown_colors(theme),
        (
            ThemeDropdown,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
}

/// Spawn the three SIZE label + numeric-field rows (C2) — width / height (`1..=MAX_GRID_SPAN`)
/// and levels (`1..=MAX_LEVELS`), each seeded from the config's current (validated) size — and
/// push them onto `rows`.
fn spawn_size_rows(
    commands: &mut Commands,
    theme: &GdtfTheme,
    config: &VizConfig,
    rows: &mut Vec<Entity>,
) {
    let size = config.grid_size().unwrap_or_default();
    rows.push(label_node(commands, "Width", ThemeRole::Text));
    rows.push(spawn_numeric_field(
        commands,
        *size.width(),
        NumericRange::new(1_u8, MAX_GRID_SPAN),
        field_colors(theme),
        (
            WidthField,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
    rows.push(label_node(commands, "Height", ThemeRole::Text));
    rows.push(spawn_numeric_field(
        commands,
        *size.height(),
        NumericRange::new(1_u8, MAX_GRID_SPAN),
        field_colors(theme),
        (
            HeightField,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
    rows.push(label_node(commands, "Levels", ThemeRole::Text));
    rows.push(spawn_numeric_field(
        commands,
        *size.levels(),
        NumericRange::new(1_u8, MAX_LEVELS),
        field_colors(theme),
        (
            LevelsField,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
}

/// Spawn the SEED label + numeric field (C3) — seeded with the config's current seed over the
/// full `u64` range — and push them onto `rows`.
fn spawn_seed_row(
    commands: &mut Commands,
    theme: &GdtfTheme,
    config: &VizConfig,
    rows: &mut Vec<Entity>,
) {
    rows.push(label_node(commands, "Seed", ThemeRole::Text));
    rows.push(spawn_numeric_field(
        commands,
        *config.seed(),
        NumericRange::new(u64::MIN, u64::MAX),
        field_colors(theme),
        (
            SeedField,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
}

/// Spawn the PLAYER + ENEMY gang label + dropdown rows (C4) — over the loaded gang registry
/// (empty when absent, the no-gang fallback), each opened on the config's current chosen gang —
/// and push them onto `rows`.
fn spawn_gang_rows(
    commands: &mut Commands,
    theme: &GdtfTheme,
    config: &VizConfig,
    gangs: Option<&GangRegistry>,
    rows: &mut Vec<Entity>,
) {
    let options = gang_options(gangs);
    rows.push(label_node(commands, "Player gang", ThemeRole::Text));
    let player_initial = config
        .player_gang()
        .map_or(0, |gang| option_index_of(&options, gang));
    rows.push(spawn_dropdown(
        commands,
        options.clone(),
        player_initial,
        dropdown_colors(theme),
        (
            PlayerGangDropdown,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
    rows.push(label_node(commands, "Enemy gang", ThemeRole::Text));
    let enemy_initial = config
        .enemy_gang()
        .map_or(0, |gang| option_index_of(&options, gang));
    rows.push(spawn_dropdown(
        commands,
        options,
        enemy_initial,
        dropdown_colors(theme),
        (
            EnemyGangDropdown,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    ));
}

/// The current size-validity status text (C2): `OK` for a valid combo, else the
/// [`GridSizeError`](gdtf_battle_sim::GridSizeError) message naming the offending axis.
pub(in crate::states::running::procgen_viz) fn size_status_text(config: &VizConfig) -> String {
    match config.grid_size() {
        Ok(size) => format!("OK {}x{}x{}", *size.width(), *size.height(), *size.levels()),
        Err(err) => err.to_string(),
    }
}

/// Build the theme dropdown's option list (C1) — every loaded theme, labelled by its display
/// name, sorted by display name for a stable order (the registry's iteration order is
/// unspecified). The option identity IS the [`ThemeUuid`]. Empty when no registry (degraded).
fn theme_options(themes: Option<&UuidThemeRegistry>) -> Vec<DropdownOption<ThemeUuid>> {
    let Some(themes) = themes else {
        return Vec::new();
    };
    let mut options: Vec<(String, ThemeUuid)> = themes
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    options.sort_by(|(a, _), (b, _)| a.cmp(b));
    options
        .into_iter()
        .map(|(label, key)| DropdownOption::new(key, label))
        .collect()
}

/// Build the gang dropdown's option list (C4) — every loaded gang, labelled by its name, sorted
/// for a stable order. The option identity IS the [`GangName`]. Empty when no registry (the
/// no-gang fallback).
fn gang_options(gangs: Option<&GangRegistry>) -> Vec<DropdownOption<GangName>> {
    let Some(gangs) = gangs else {
        return Vec::new();
    };
    let mut keys: Vec<GangName> = gangs.keys().cloned().collect();
    keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    keys.into_iter()
        .map(|key| {
            let label = key.as_str().to_owned();
            DropdownOption::new(key, label)
        })
        .collect()
}

/// The index of `id` in `options` (matched by identity), or `0` if absent — the dropdown's
/// initially-shown slot.
fn option_index_of<T: gdtf_ui::OptionId>(options: &[DropdownOption<T>], id: &T) -> usize {
    options
        .iter()
        .position(|opt| opt.id() == id)
        .unwrap_or_default()
}

/// Spawn a small themed label node and return it.
fn label_node(commands: &mut Commands, text: &str, role: ThemeRole) -> Entity {
    commands
        .spawn((
            Themed::new(role),
            Text::new(text.to_owned()),
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ))
        .id()
}

/// The [`DropdownColors`] an input-panel dropdown paints with, from the theme. Pure UI plumbing
/// (the gang-editor `dropdown_colors` precedent).
fn dropdown_colors(theme: &GdtfTheme) -> DropdownColors {
    DropdownColors {
        control_bg:          *theme.button.color,
        text:                *theme.text.text_color,
        popup_bg:            *theme.panel.color,
        option_bg:           *theme.button.color,
        option_highlight_bg: *theme.button.hover,
    }
}

/// The [`FieldColors`] an input-panel numeric field paints with, from the theme. Pure UI
/// plumbing (the gang-editor `field_colors` precedent).
fn field_colors(theme: &GdtfTheme) -> FieldColors {
    FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    }
}
