//! The THEME-mode form **layout** (GTW-475): spawn the form's widgets into the four regions'
//! THEME-mode content containers `OnEnter(Editing)`.
//!
//! Reuses the landed `gdtf_ui` widgets: [`spawn_text_field`] (display name), [`spawn_dropdown`]
//! (default-floor picker), [`spawn_progress_bar`] (the resolved-stats HP bar — C3), and plain
//! [`Button`] rows (the terrain-library multi-select + the save / new-theme buttons). The widgets
//! parent into the THEME container under each region so they show only in THEME mode (the mode
//! toggle flips the container's [`Visibility`]).
//!
//! LAYOUT (logged sub-decisions, mirroring `terrain_form` spawn.rs):
//! - LEFT region (THEME container) = the TERRAIN-LIBRARY multi-select — a column of `Button`
//!   rows, one per terrain def in the [`TerrainDefRegistry`], each showing the def display name +
//!   its sim kind; a selected row carries [`ActiveButton`](gdtf_ui::ActiveButton) (the
//!   `terrain_form` tag-toggle precedent). This is the C2 "multi-select of terrain chosen from the
//!   `TerrainDefRegistry` library".
//! - CENTER region (THEME container) = the theme-global control: the DEFAULT-FLOOR dropdown (C6).
//! - RIGHT region (THEME container) = the metadata stack: display name, the read-only KEY, the
//!   resolved-stats readout (C3), and the New-theme + Save-theme buttons.
//! - STAT region (THEME container) = the read-only `.terrain_theme.ron` live PREVIEW (C3).

use bevy::{prelude::*, ui::FlexDirection};
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainSimKind, TerrainUuid};
use gdtf_ui::{
    CommittedTextValue, DropdownColors, FieldColors, FillFraction, spawn_dropdown,
    spawn_progress_bar, spawn_text_field, theme::GdtfTheme,
};

use super::types::{
    NewThemeButton, SaveThemeButton, ThemeDefaultFloorPicker, ThemeDraft, ThemeKeyText,
    ThemeNameField, ThemeResolvedHpBar, ThemeResolvedStatsText, ThemeRonPreview, ThemeTerrainRow,
};
use crate::{
    CanvasRegion, LeftPaletteRegion, RightPanelRegion, StatRegion, mode::ThemeModeContent,
    mode_host::mode_host_under_region,
};

/// `OnEnter(Editing)`: spawn the THEME form's widgets into the four regions' THEME-mode content
/// containers (GTW-475 C2).
///
/// Gated on the live [`GdtfTheme`] (for the control colors). Seeds the widgets from a fresh
/// [`ThemeDraft::default`] (the draft is inserted in the same `OnEnter` buffer, so seeding from
/// the default avoids the command-flush race — the GTW-421/474 precedent). The terrain-library
/// rows are NOT spawned here — they depend on the [`TerrainDefRegistry`] (inserted via a deferred
/// command on `OnEnter`), so [`sync_theme_library`](super::systems::sync_theme_library) builds
/// them in `Update` once the registry resolves (the GTW-422 palette-sync precedent). Each widget
/// carries its identity marker so the form's drive systems map a commit to the draft.
pub(crate) fn spawn_theme_form(mut commands: Commands, theme: Res<GdtfTheme>) {
    let draft = ThemeDraft::default();
    let field_colors = FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    };
    let dropdown_colors = DropdownColors {
        control_bg:          *theme.panel.color,
        text:                *theme.text.text_color,
        popup_bg:            *theme.panel.color,
        option_bg:           *theme.panel.border_color,
        option_highlight_bg: *theme.button.hover,
    };
    let label_color = *theme.text.text_color;
    let button_bg = *theme.panel.border_color;
    let bar_remaining = *theme.button.hover;
    let bar_lost = *theme.panel.border_color;

    spawn_left_library_header(&mut commands, label_color);
    spawn_center_default_floor(&mut commands, dropdown_colors, label_color);
    spawn_right_metadata(
        &mut commands,
        field_colors,
        label_color,
        button_bg,
        bar_remaining,
        bar_lost,
        &draft,
    );
    spawn_stat_preview(&mut commands, label_color);
}

/// LEFT region: the terrain-library multi-select HEADER (the rows themselves are built by
/// [`sync_theme_library`](super::systems::sync_theme_library) once the registry resolves). The
/// header anchors the column the rows parent into.
fn spawn_left_library_header(commands: &mut Commands, label_color: Color) {
    let header = commands
        .spawn((
            Text::new("Terrain library (click to add/remove)"),
            TextColor(label_color),
            header_node(),
        ))
        .id();
    let column = commands.spawn(form_column_node()).add_child(header).id();
    parent_under_theme::<LeftPaletteRegion>(commands, column);
}

/// CENTER region: the DEFAULT-FLOOR dropdown (C6). Spawned EMPTY (no options) — the options are
/// the theme's OWN selected terrain (Slab-kind preferred), so
/// [`sync_default_floor_options`](super::systems::sync_default_floor_options) repopulates it as
/// the multi-select changes. Until a terrain is selected the dropdown offers nothing.
fn spawn_center_default_floor(
    commands: &mut Commands,
    dropdown_colors: DropdownColors,
    label_color: Color,
) {
    let dropdown = spawn_dropdown::<TerrainUuid>(
        commands,
        Vec::new(),
        0,
        dropdown_colors,
        ThemeDefaultFloorPicker,
    );
    let group = labeled(
        commands,
        "Default floor (one of this theme's terrain)",
        label_color,
        dropdown,
    );
    let column = commands.spawn(form_column_node()).add_child(group).id();
    parent_under_theme::<CanvasRegion>(commands, column);
}

/// RIGHT region: the metadata stack (display name, read-only KEY, the C3 resolved-stats readout,
/// the New-theme + Save-theme buttons).
fn spawn_right_metadata(
    commands: &mut Commands,
    field_colors: FieldColors,
    label_color: Color,
    button_bg: Color,
    bar_remaining: Color,
    bar_lost: Color,
    draft: &ThemeDraft,
) {
    let name = spawn_text_field(
        commands,
        CommittedTextValue::new(draft.display_name()),
        field_colors,
        ThemeNameField,
    );
    let name_group = labeled(commands, "Theme name", label_color, name);

    let key_text = commands
        .spawn((
            ThemeKeyText,
            Text::new(format!("Key: {}", *draft.key())),
            TextColor(label_color),
            header_node(),
        ))
        .id();

    let stats_group = spawn_resolved_stats_group(commands, label_color, bar_remaining, bar_lost);

    let new_button = spawn_button(
        commands,
        "New theme",
        NewThemeButton,
        button_bg,
        label_color,
    );
    let save_button = spawn_button(
        commands,
        "Save theme",
        SaveThemeButton,
        button_bg,
        label_color,
    );

    let column = commands
        .spawn(form_column_node())
        .add_children(&[name_group, key_text, stats_group, new_button, save_button])
        .id();
    parent_under_theme::<RightPanelRegion>(commands, column);
}

/// The C3 RESOLVED-STATS group — the single-source-of-truth proof: a header, a read-only text
/// readout ([`ThemeResolvedStatsText`]) and an HP [`ProgressBar`](gdtf_ui::ProgressBarTrack)
/// ([`ThemeResolvedHpBar`]) the form RESOLVES for the selected default-floor terrain (it stores a
/// UUID; the stats are resolved against the [`TerrainDefRegistry`], never inlined).
fn spawn_resolved_stats_group(
    commands: &mut Commands,
    label_color: Color,
    bar_remaining: Color,
    bar_lost: Color,
) -> Entity {
    let header = commands
        .spawn((
            Text::new("Resolved default-floor stats"),
            TextColor(label_color),
            header_node(),
        ))
        .id();
    let readout = commands
        .spawn((
            ThemeResolvedStatsText,
            Text::new("Pick a default floor to resolve its stats."),
            TextColor(label_color),
            header_node(),
        ))
        .id();
    let hp_bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        bar_remaining,
        bar_lost,
        ThemeResolvedHpBar,
    );
    commands
        .spawn(group_node())
        .add_children(&[header, readout, hp_bar])
        .id()
}

/// STAT region: the read-only `.terrain_theme.ron` live preview text node (C3).
fn spawn_stat_preview(commands: &mut Commands, label_color: Color) {
    let preview = commands
        .spawn((
            ThemeRonPreview,
            Text::new(".terrain_theme.ron preview"),
            TextColor(label_color),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Vh(1.0)),
                ..default()
            },
        ))
        .id();
    parent_under_theme::<StatRegion>(commands, preview);
}

/// Spawn one full-width labeled button carrying `marker`.
fn spawn_button(
    commands: &mut Commands,
    label: &str,
    marker: impl Bundle,
    button_bg: Color,
    label_color: Color,
) -> Entity {
    commands
        .spawn((
            marker,
            Button,
            BackgroundColor(button_bg),
            Node {
                width: Val::Percent(100.0),
                margin: UiRect::top(Val::Vh(0.6)),
                padding: UiRect::all(Val::Vh(0.6)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child((Text::new(label), TextColor(label_color)))
        .id()
}

/// Build ONE terrain-library row entity — a clickable [`Button`] showing the def's display name +
/// its sim kind, carrying its [`ThemeTerrainRow`] (its [`TerrainUuid`]) for the toggle handler.
/// The selected rows carry [`ActiveButton`](gdtf_ui::ActiveButton). Used by
/// [`sync_theme_library`](super::systems::sync_theme_library), so it lives here beside the layout
/// helpers (the `terrain_form` `spawn_tags_group` row precedent).
pub(super) fn spawn_terrain_library_row(
    commands: &mut Commands,
    key: TerrainUuid,
    def: &TerrainDef,
    selected: bool,
    row_bg: Color,
    text_color: Color,
) -> Entity {
    let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
    let mut row = commands.spawn((
        Button,
        ThemeTerrainRow::new(key),
        BackgroundColor(row_bg),
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::all(Val::Vh(0.5)),
            margin: UiRect::bottom(Val::Vh(0.3)),
            ..default()
        },
    ));
    row.with_child((Text::new(label), TextColor(text_color)));
    if selected {
        row.insert(gdtf_ui::ActiveButton);
    }
    row.id()
}

/// The short human label for a terrain sim kind (Wall / Cover / Slab) — shown beside each
/// library row's name so the author sees the structural kind at a glance (C2).
#[must_use]
pub(super) const fn sim_kind_label(kind: &TerrainSimKind) -> &'static str {
    match kind {
        TerrainSimKind::Wall { .. } => "Wall",
        TerrainSimKind::Cover { .. } => "Cover",
        TerrainSimKind::Slab { .. } => "Slab",
    }
}

/// Wrap `control` in a labeled COLUMN group (a small text label above the control), stretched to
/// the panel width — the `terrain_form` `labeled` shape.
fn labeled(commands: &mut Commands, label: &str, color: Color, control: Entity) -> Entity {
    let label_node = commands
        .spawn((Text::new(label), TextColor(color), header_node()))
        .id();
    commands
        .spawn(group_node())
        .add_children(&[label_node, control])
        .id()
}

/// Parent `content` under the THEME-mode content container of the region marked `Region` (a
/// deferred command, the shell-parenting idiom — the `terrain_form` `parent_under_terrain` twin).
fn parent_under_theme<Region: Component>(commands: &mut Commands, content: Entity) {
    commands.queue(move |world: &mut World| {
        let Some(host) = mode_host_under_region::<Region, ThemeModeContent>(world) else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(content);
        }
    });
}

/// The form content COLUMN [`Node`]: full-width, top-aligned, padded, with row gaps so groups
/// breathe. Relative units (no fixed `Px`).
fn form_column_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Start,
        padding: UiRect::all(Val::Vh(1.0)),
        row_gap: Val::Vh(1.0),
        ..default()
    }
}

/// One labeled GROUP's [`Node`]: a flex COLUMN (label above control) filling the width.
fn group_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: Val::Vh(0.4),
        ..default()
    }
}

/// A small header/label [`Node`]: a little bottom margin so the label sits above its control.
fn header_node() -> Node {
    Node {
        margin: UiRect::bottom(Val::Vh(0.3)),
        ..default()
    }
}
