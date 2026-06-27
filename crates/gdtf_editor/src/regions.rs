//! The four empty themed layout regions of the map-editor shell (GTW-417 AC2).
//!
//! The shell lays out FOUR regions that later children (GTW-421 right panel, GTW-422 left
//! palette, GTW-423 canvas) fill: a scrollable RIGHT panel, a scrollable LEFT palette, a
//! central CANVAS region, and a BOTTOM-RIGHT stat region. They are spawned EMPTY here —
//! themed containers only — using the `gdtf_ui` widgets: [`spawn_scroll_list`] for the two
//! eventually-scrollable panels (right + left palette) and [`spawn_panel`] for the static
//! regions (canvas + stat). Sizing is RESPONSIVE (Percent / flex, no fixed `Px`) per the
//! ui-responsive rule, so the layout tracks the window.
//!
//! Each region carries a named unit MARKER (no-bare-types rule) so later children — and
//! the GTW-417 test — can find it without depending on tree shape.

use bevy::{
    prelude::*,
    ui::{FlexDirection, PositionType, Val},
};
use gdtf_ui::{ScrollListColors, spawn_panel, spawn_scroll_list, theme::GdtfTheme};

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

/// Marker on the editor shell's full-viewport root node — the flex row that arranges the
/// left palette, the centre column, and the right column. A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct EditorShellRoot;

/// Fraction of viewport WIDTH the left palette column occupies.
const LEFT_PALETTE_WIDTH_PCT: f32 = 18.0;

/// Fraction of viewport WIDTH the right column (right panel + stat region) occupies.
const RIGHT_COLUMN_WIDTH_PCT: f32 = 24.0;

/// Fraction of the right column's HEIGHT the bottom-right stat region occupies (the right
/// panel takes the remaining flex space above it).
const STAT_REGION_HEIGHT_PCT: f32 = 30.0;

/// The neutral scroll-list color set for the two empty scrollable regions, derived from
/// the live [`GdtfTheme`] panel sub-theme so the scroll viewports match the panels' look
/// (the area uses the panel fill; the track/thumb derive from its border for contrast).
fn scroll_colors(theme: &GdtfTheme) -> ScrollListColors {
    ScrollListColors {
        area:  *theme.panel.color,
        track: *theme.panel.border_color,
        thumb: *theme.panel.border_color,
    }
}

/// `OnEnter(EditorState::Editing)`: spawns the camera and the four empty themed regions.
///
/// Builds the full-viewport [`EditorShellRoot`] flex ROW, then under it:
/// - the [`LeftPaletteRegion`] scroll list (left column, [`LEFT_PALETTE_WIDTH_PCT`] wide),
/// - the central [`CanvasRegion`] panel (the flex-grow middle column),
/// - a right column ([`RIGHT_COLUMN_WIDTH_PCT`] wide) holding the [`RightPanelRegion`]
///   scroll list (flex-grow, top) above the [`StatRegion`] panel
///   ([`STAT_REGION_HEIGHT_PCT`] tall, bottom).
///
/// All sizing is relative (Percent / flex) — no fixed `Px` — so the shell tracks the
/// window. The regions are spawned EMPTY (later children fill them).
pub(crate) fn spawn_editor_shell(mut commands: Commands, theme: Res<GdtfTheme>) {
    // The editor's own 2D camera (the editor never reuses a game camera).
    commands.spawn(Camera2d);

    // Full-viewport root flex ROW: palette | centre | right column.
    let root = commands
        .spawn((
            EditorShellRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
        ))
        .id();

    // LEFT palette — a scrollable region. `spawn_scroll_list` parents the returned area
    // under its OWN root frame; size + place that root by wrapping the whole list in a
    // sized column cell, then parent the list root under it.
    let palette_cell = spawn_column_cell(&mut commands, Val::Percent(LEFT_PALETTE_WIDTH_PCT), None);
    let palette_area = spawn_scroll_list(&mut commands, scroll_colors(&theme), LeftPaletteRegion);
    parent_scroll_root_under(&mut commands, palette_cell, palette_area);
    commands.entity(root).add_child(palette_cell);

    // CENTRE canvas — a themed panel that grows to fill the middle column.
    let canvas = spawn_panel(&mut commands, &theme);
    commands.entity(canvas).insert((
        CanvasRegion,
        Node {
            flex_grow: 1.0,
            height: Val::Percent(100.0),
            ..default()
        },
    ));
    commands.entity(root).add_child(canvas);

    // RIGHT column — a flex COLUMN holding the right panel (top, flex-grow) above the
    // bottom-right stat region.
    let right_column = spawn_column_cell(
        &mut commands,
        Val::Percent(RIGHT_COLUMN_WIDTH_PCT),
        Some(FlexDirection::Column),
    );

    // RIGHT panel — a scrollable region filling the top of the right column.
    let right_panel_cell = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            ..default()
        })
        .id();
    let right_panel_area =
        spawn_scroll_list(&mut commands, scroll_colors(&theme), RightPanelRegion);
    parent_scroll_root_under(&mut commands, right_panel_cell, right_panel_area);

    // BOTTOM-RIGHT stat region — a themed panel pinned to the bottom of the right column.
    let stat = spawn_panel(&mut commands, &theme);
    commands.entity(stat).insert((
        StatRegion,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(STAT_REGION_HEIGHT_PCT),
            ..default()
        },
    ));

    commands
        .entity(right_column)
        .add_children(&[right_panel_cell, stat]);
    commands.entity(root).add_child(right_column);
}

/// Spawns a sized column-cell [`Node`] of the given relative `width` and full height, with
/// an optional [`FlexDirection`] (defaulting to a column when `dir` is given). Used to wrap
/// the scroll lists and to build the right column.
fn spawn_column_cell(commands: &mut Commands, width: Val, dir: Option<FlexDirection>) -> Entity {
    commands
        .spawn(Node {
            width,
            height: Val::Percent(100.0),
            flex_direction: dir.unwrap_or(FlexDirection::Column),
            ..default()
        })
        .id()
}

/// Re-parents a [`spawn_scroll_list`] ROOT frame under `cell` so the list fills it.
///
/// `spawn_scroll_list` returns the SCROLL AREA and parents it under a root frame within the
/// same command buffer; this queues a deferred command that, once that parenting has
/// applied, moves the area's root under `cell` (the `scroll_list_demo` precedent).
fn parent_scroll_root_under(commands: &mut Commands, cell: Entity, area: Entity) {
    commands.queue(move |world: &mut World| {
        if let Some(root) = world.get::<ChildOf>(area).map(ChildOf::parent)
            && let Ok(mut cell_entity) = world.get_entity_mut(cell)
        {
            cell_entity.add_child(root);
        }
    });
}
