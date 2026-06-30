//! The Workbench shell: a full-width TOP BAR + the verbatim four-region row + a STATUS BAR
//! (GTW-417 four regions; GTW-474 Workbench shell).
//!
//! GTW-417 laid out FOUR regions that children fill: a scrollable RIGHT panel, a scrollable
//! LEFT palette, a central CANVAS region, and a BOTTOM-RIGHT stat region. GTW-474 grows the
//! editor into a multi-mode "Workbench" tool by KEEPING those four region markers + their
//! proportions verbatim and wrapping them in a flex COLUMN: a full-width [`EditorTopBar`] above
//! the region row (mode tabs + the promoted global theme dropdown) and a thin
//! [`EditorStatusBar`] below it. The top bar + status bar carry a [`GlobalZIndex`] strictly
//! above the columns so they actually draw (a Visible, laid-out, on-screen, sized node still
//! draws NOTHING when an opaque higher-z sibling paints over it — bevy-traps #8 / GTW-294).
//!
//! Per the Workbench design, each region hosts ONE per-mode content container per editor mode
//! ([`PrefabModeContent`](crate::mode::PrefabModeContent) /
//! [`TerrainModeContent`](crate::mode::TerrainModeContent)), all spawned once and toggled by
//! [`Visibility`] (never despawn). The existing painter's content parents into the PREFAB
//! container; the GTW-474 terrain form parents into the TERRAIN container.
//!
//! Each region carries a named unit MARKER (no-bare-types rule) so children — and the tests —
//! find it without depending on tree shape.

use bevy::{
    prelude::*,
    ui::{AlignItems, FlexDirection, JustifyContent, PositionType, Val},
};
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};
use gdtf_ui::{
    DropdownColors, DropdownOption, Orientation, ScrollListColors, SegmentColors, spawn_dropdown,
    spawn_panel, spawn_scroll_list, spawn_segmented_control, theme::GdtfTheme,
};

use crate::{
    mode::{EditorMode, EditorModeTabs, PrefabModeContent, TerrainModeContent, ThemeModeContent},
    right_panel::ThemeDropdown,
};

/// Marker on the scrollable RIGHT panel region (the GTW-421 right-panel home).
///
/// A unit marker — presence alone is the signal. Carried by the `gdtf_ui`
/// [`ScrollList`](gdtf_ui::ScrollList) root frame so children/tests can find this region.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RightPanelRegion;

/// Marker on the scrollable LEFT palette region (the GTW-422 palette home).
///
/// A unit marker — presence alone is the signal. Carried by the `gdtf_ui`
/// [`ScrollList`](gdtf_ui::ScrollList) root frame so children/tests can find this region.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LeftPaletteRegion;

/// Marker on the central CANVAS region (the GTW-423 canvas home).
///
/// A unit marker — presence alone is the signal. Carried by a themed
/// [`spawn_panel`] container so children/tests can find this region.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasRegion;

/// Marker on the BOTTOM-RIGHT stat region (the selected-tile / brush stats home).
///
/// A unit marker — presence alone is the signal. Carried by a themed
/// [`spawn_panel`] container so children/tests can find this region.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatRegion;

/// Marker on the editor shell's full-viewport root node — the flex COLUMN that stacks the top
/// bar, the four-region row, and the status bar (GTW-474). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct EditorShellRoot;

/// Marker on the full-width TOP BAR — the [`GlobalZIndex`]'d strip holding the mode tabs (left)
/// and the promoted global theme dropdown (right) (GTW-474). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct EditorTopBar;

/// Marker on the thin STATUS BAR pinned under the region row — a [`GlobalZIndex`]'d strip with
/// mutated status [`Text`] (GTW-474). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct EditorStatusBar;

/// Marker on the status bar's status [`Text`] node — the single line
/// [`refresh_status_bar`](crate::mode_status::refresh_status_bar) rewrites with the current
/// mode + theme (GTW-474). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatusText;

/// Fraction of viewport WIDTH the left palette column occupies.
const LEFT_PALETTE_WIDTH_PCT: f32 = 18.0;

/// Fraction of viewport WIDTH the right column (right panel + stat region) occupies.
const RIGHT_COLUMN_WIDTH_PCT: f32 = 30.0;

/// Fraction of the right column's HEIGHT the bottom-right stat region occupies (the right
/// panel takes the remaining flex space above it).
const STAT_REGION_HEIGHT_PCT: f32 = 30.0;

/// The [`GlobalZIndex`] band the top bar + status bar render on — strictly above the columns'
/// default `0`, so the opaque bars are never occluded and (crucially) their children DRAW
/// (bevy-traps #8 / GTW-294). The promoted theme dropdown's popup floats above this on the
/// `gdtf_ui` dropdown popup z (`30`), so the open list is never hidden behind a column.
const SHELL_BAR_Z: i32 = 10;

/// The neutral scroll-list color set for the two scrollable regions, derived from the live
/// [`GdtfTheme`] panel sub-theme.
fn scroll_colors(theme: &GdtfTheme) -> ScrollListColors {
    ScrollListColors {
        area:  *theme.panel.color,
        track: *theme.panel.border_color,
        thumb: *theme.panel.border_color,
    }
}

/// `OnEnter(EditorState::Editing)`: spawn the camera and the full Workbench shell (GTW-474).
///
/// Builds the full-viewport [`EditorShellRoot`] flex COLUMN, then under it, top to bottom:
/// - the full-width [`EditorTopBar`] ([`GlobalZIndex`]'d) with the mode tabs (left) + the
///   promoted global theme dropdown (right),
/// - the four-region ROW (verbatim GTW-417 proportions), with per-mode content containers
///   under each region so a mode switch toggles [`Visibility`] (never despawn),
/// - the thin [`EditorStatusBar`] ([`GlobalZIndex`]'d) with a mutated status [`Text`].
///
/// All sizing is relative (Percent / flex / Vh) — no fixed `Px` — so the shell tracks the
/// window. The regions are spawned EMPTY apart from their per-mode containers (later children
/// fill them).
pub(crate) fn spawn_editor_shell(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    themes: Option<Res<UuidThemeRegistry>>,
) {
    // The editor's own 2D camera (the editor never reuses a game camera).
    commands.spawn(Camera2d);

    // Full-viewport root flex COLUMN: top bar / region row / status bar.
    let root = commands
        .spawn((
            EditorShellRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    let top_bar = spawn_top_bar(&mut commands, &theme, themes.as_deref());
    let region_row = spawn_region_row(&mut commands, &theme);
    let status_bar = spawn_status_bar(&mut commands, &theme);

    commands
        .entity(root)
        .add_children(&[top_bar, region_row, status_bar]);
}

/// Spawn the full-width TOP BAR: the mode-tabs segmented control (left) + the promoted global
/// theme dropdown (right). Carries a [`GlobalZIndex`] so it (and its children) draw above the
/// columns (bevy-traps #8). Returns the bar root.
fn spawn_top_bar(
    commands: &mut Commands,
    theme: &GdtfTheme,
    themes: Option<&UuidThemeRegistry>,
) -> Entity {
    let segment_colors = SegmentColors {
        active_bg:   *theme.button.hover,
        active_text: *theme.text.text_color,
        base_bg:     *theme.panel.color,
        base_text:   *theme.text.text_color,
    };
    // The mode tabs — one segment per editor mode, the default mode pre-selected (C1). The
    // segmented control emits `SegmentSelected`; `apply_mode_switch` maps it to `EditorMode`.
    let tabs = spawn_segmented_control(
        commands,
        &EditorMode::tab_labels(),
        EditorMode::default().tab_index(),
        segment_colors,
        Orientation::Horizontal,
        EditorModeTabs,
    );

    // The promoted GLOBAL theme dropdown (C1) — moved out of the right panel into the top bar so
    // the active theme is global + visible in every mode. Its options are the registered themes
    // by display name (sorted for determinism), the first pre-selected.
    let dropdown_colors = DropdownColors {
        control_bg:          *theme.panel.color,
        text:                *theme.text.text_color,
        popup_bg:            *theme.panel.color,
        option_bg:           *theme.panel.border_color,
        option_highlight_bg: *theme.button.hover,
    };
    let options = theme_options(themes);
    let dropdown = spawn_dropdown(commands, options, 0, dropdown_colors, ThemeDropdown);
    let theme_label = commands
        .spawn((
            Text::new("Theme"),
            TextColor(*theme.text.text_color),
            Node {
                margin: UiRect::right(Val::Vw(0.6)),
                ..default()
            },
        ))
        .id();
    let theme_group = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[theme_label, dropdown])
        .id();

    commands
        .spawn((
            EditorTopBar,
            GlobalZIndex(SHELL_BAR_Z),
            BackgroundColor(*theme.panel.color),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Vw(0.8), Val::Vh(0.6)),
                column_gap: Val::Vw(0.8),
                ..default()
            },
        ))
        .add_children(&[tabs, theme_group])
        .id()
}

/// Spawn the four-region ROW (verbatim GTW-417 proportions), grows to fill the height between
/// the top bar and the status bar. Each region gets its per-mode content containers. Returns the
/// row root.
fn spawn_region_row(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let row = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .id();

    // LEFT palette — a scrollable region.
    let palette_cell = spawn_column_cell(commands, Val::Percent(LEFT_PALETTE_WIDTH_PCT), None);
    let palette_area = spawn_scroll_list(commands, scroll_colors(theme), LeftPaletteRegion);
    spawn_mode_hosts_under(commands, palette_area);
    parent_scroll_root_under(commands, palette_cell, palette_area);
    commands.entity(row).add_child(palette_cell);

    // CENTRE canvas — a themed panel that grows to fill the middle column.
    let canvas = spawn_panel(commands, theme);
    commands.entity(canvas).insert((
        CanvasRegion,
        Node {
            flex_grow: 1.0,
            height: Val::Percent(100.0),
            ..default()
        },
    ));
    spawn_mode_hosts_under(commands, canvas);
    commands.entity(row).add_child(canvas);

    // RIGHT column — a flex COLUMN holding the right panel (top, flex-grow) above the
    // bottom-right stat region.
    let right_column = spawn_column_cell(
        commands,
        Val::Percent(RIGHT_COLUMN_WIDTH_PCT),
        Some(FlexDirection::Column),
    );

    // RIGHT panel — a scrollable region filling the top of the right column. `min_height: 0`
    // lets the cell shrink so the scroll list fills it from the TOP (the GTW-421 bottom-cramp
    // fix).
    let right_panel_cell = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            min_height: Val::Px(0.0),
            ..default()
        })
        .id();
    let right_panel_area = spawn_scroll_list(commands, scroll_colors(theme), RightPanelRegion);
    spawn_mode_hosts_under(commands, right_panel_area);
    parent_scroll_root_under(commands, right_panel_cell, right_panel_area);

    // BOTTOM-RIGHT stat region — a themed panel pinned to the bottom of the right column.
    let stat = spawn_panel(commands, theme);
    commands.entity(stat).insert((
        StatRegion,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(STAT_REGION_HEIGHT_PCT),
            ..default()
        },
    ));
    spawn_mode_hosts_under(commands, stat);

    commands
        .entity(right_column)
        .add_children(&[right_panel_cell, stat]);
    commands.entity(row).add_child(right_column);

    row
}

/// Spawn the thin STATUS BAR pinned under the region row, with a mutated status [`Text`].
/// Carries a [`GlobalZIndex`] so it draws above the columns (bevy-traps #8). Returns the bar
/// root.
fn spawn_status_bar(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let status = commands
        .spawn((
            StatusText,
            Text::new("Editing"),
            TextColor(*theme.text.text_color),
        ))
        .id();
    commands
        .spawn((
            EditorStatusBar,
            GlobalZIndex(SHELL_BAR_Z),
            BackgroundColor(*theme.panel.color),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Vw(0.8), Val::Vh(0.4)),
                ..default()
            },
        ))
        .add_child(status)
        .id()
}

/// Spawn the THREE per-mode content containers ([`PrefabModeContent`] +
/// [`TerrainModeContent`] + [`ThemeModeContent`]) as children of a region's content `host` (the
/// scroll AREA for the scrollable regions, the panel for the static ones), each a full-size flex
/// column (GTW-474 two modes; GTW-475 adds the THEME container).
///
/// All three are spawned once with explicit [`Visibility`]: the PREFAB container is
/// [`Visible`](Visibility::Inherited) (the default [`EditorMode::Prefab`]) and the TERRAIN +
/// THEME ones [`Hidden`](Visibility::Hidden), so the editor opens in the prefab painter and
/// [`toggle_mode_content`](crate::mode::toggle_mode_content) flips them on a mode switch
/// (mutate-in-place, never despawn). Content the existing painter / the terrain form / the theme
/// form spawns parents into the matching container, so it inherits the container's visibility.
fn spawn_mode_hosts_under(commands: &mut Commands, host: Entity) {
    let prefab = commands
        .spawn((PrefabModeContent, Visibility::Inherited, mode_host_node()))
        .id();
    let terrain = commands
        .spawn((TerrainModeContent, Visibility::Hidden, mode_host_node()))
        .id();
    let theme = commands
        .spawn((ThemeModeContent, Visibility::Hidden, mode_host_node()))
        .id();
    commands
        .entity(host)
        .add_children(&[prefab, terrain, theme]);
}

/// One per-mode content container's [`Node`]: a full-width, top-aligned flex COLUMN that holds a
/// mode's content for one region. Relative units (no fixed `Px`).
fn mode_host_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Start,
        ..default()
    }
}

/// Build the global theme dropdown's option list from the [`UuidThemeRegistry`] — one
/// [`DropdownOption<ThemeUuid>`] per registered theme, labeled by its display name, sorted by
/// label so the order is deterministic (the registry is a `HashMap`). An absent / empty registry
/// yields an empty list (the dropdown then offers nothing rather than panicking).
fn theme_options(themes: Option<&UuidThemeRegistry>) -> Vec<DropdownOption<ThemeUuid>> {
    let Some(themes) = themes else {
        return Vec::new();
    };
    let mut options: Vec<(String, ThemeUuid)> = themes
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    options.sort_by(|a, b| a.0.cmp(&b.0));
    options
        .into_iter()
        .map(|(label, key)| DropdownOption::new(key, label))
        .collect()
}

/// Spawns a sized column-cell [`Node`] of the given relative `width` and full height, with an
/// optional [`FlexDirection`] (defaulting to a column when `dir` is given). `justify_content:
/// Start` pins the cell's children to the TOP of its main axis (the GTW-421 bottom-cramp guard).
fn spawn_column_cell(commands: &mut Commands, width: Val, dir: Option<FlexDirection>) -> Entity {
    commands
        .spawn(Node {
            width,
            height: Val::Percent(100.0),
            flex_direction: dir.unwrap_or(FlexDirection::Column),
            justify_content: JustifyContent::Start,
            ..default()
        })
        .id()
}

/// Re-parents a [`spawn_scroll_list`] ROOT frame under `cell` so the list fills it (the
/// `scroll_list_demo` precedent — `spawn_scroll_list` parents the area under its own root frame
/// in the same buffer; this deferred command moves that root under `cell`).
fn parent_scroll_root_under(commands: &mut Commands, cell: Entity, area: Entity) {
    commands.queue(move |world: &mut World| {
        if let Some(root) = world.get::<ChildOf>(area).map(ChildOf::parent)
            && let Ok(mut cell_entity) = world.get_entity_mut(cell)
        {
            cell_entity.add_child(root);
        }
    });
}
