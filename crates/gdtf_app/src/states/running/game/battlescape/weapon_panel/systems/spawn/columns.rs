//! The weapon cluster's 2x2 grid assembly: the LEFT column (Combined + Firemode) and
//! the RIGHT column (Item + Aim). Split out of the monolithic `spawn.rs` (GTW-583);
//! the authoritative layout doc lives on the parent `spawn` module.

use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, template_value},
    ui::{Node, Val},
};
use gdtf_ui::theme::GdtfTheme;

use super::{
    combined::spawn_combined_panel,
    geometry::{BOTTOM_CELL_PCT, GAP_VH, GAP_VW, LEFT_COL_PCT, RIGHT_COL_PCT, TOP_CELL_PCT},
    item_aim_panels::{spawn_aim_label, spawn_frame, spawn_item_panel},
};
use crate::states::running::game::battlescape::{
    action_bar::{spawn_aim_button, spawn_mode_panel},
    weapon_panel::components::AimPanel,
};

/// Builds the LEFT column (3/4 width) — the [`CombinedWeaponPanel`](crate::states::running::game::battlescape::weapon_panel::components::CombinedWeaponPanel) (top 3/4 height) over the
/// relocated Firemode panel ([`spawn_mode_panel`], bottom 1/4 height). Returns the column
/// [`Entity`].
pub(super) fn spawn_left_column(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let combined = spawn_combined_panel(
        commands,
        theme,
        Val::Percent(100.0),
        Val::Percent(TOP_CELL_PCT),
    );
    // The Firemode panel = the relocated action-bar Mode 3-toggle column (GTW-298). It fills the
    // bottom 1/4 cell; `rebuild_mode_buttons` shows the offered modes + the panel root.
    //
    // `spawn_mode_panel` already lays the panel out as a full-size `Row` that CLIPS its toggle row
    // (`overflow: Hidden`, so a wide label never overflows the cell into a sibling — item 8). We
    // only need to constrain its HEIGHT to the bottom 1/4 cell here, so we MUTATE just that field
    // on the existing `Node` rather than re-inserting a fresh one — a wholesale `insert(Node {
    // ..default() })` would silently DROP the panel's `overflow: Hidden` clip (and its `Row`
    // direction), letting the toggles overflow + overlap.
    let firemode = spawn_mode_panel(commands, theme);
    commands
        .entity(firemode)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.height = Val::Percent(BOTTOM_CELL_PCT);
        });
    // GTW-322 — a plain layout `Node` column (no markers); composed via `template_value`.
    let column_node = Node {
        width: Val::Percent(LEFT_COL_PCT),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    };
    let column = commands.spawn_scene(template_value(column_node)).id();
    commands.entity(column).add_children(&[combined, firemode]);
    column
}

/// Builds the RIGHT column (1/4 width) — the [`WeaponItemPanel`](crate::states::running::game::battlescape::weapon_panel::components::WeaponItemPanel) (top 3/4 height) over the
/// [`AimPanel`] (bottom 1/4 height) wrapping the relocated Aim toggle. Returns the column
/// [`Entity`].
pub(super) fn spawn_right_column(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let items = spawn_item_panel(
        commands,
        theme,
        Val::Percent(100.0),
        Val::Percent(TOP_CELL_PCT),
    );
    // The Aim panel = a framed cell wrapping the relocated action-bar Aim toggle (GTW-298),
    // laid out as a ROW: an "Aim" caption ([`AimLabel`]) on the LEFT, the toggle `Switch` on the
    // RIGHT — matching the mockup's "AIM [switch]" reading (the GTW-277 widget migration had
    // dropped the caption, leaving the bare switch). A ROW (not a column) was chosen because the
    // cell is the SHORT bottom 1/4-height of the right column: a horizontal "Aim [switch]" reads
    // cleaner in a short-and-wide box than stacking the caption over the already-horizontal
    // switch would in the tight vertical room. The cell centres the row
    // (`justify_content`/`align_items: Center`) with a small inter-child gap so the caption + the
    // self-contained switch (sized by its own track geometry, NOT a fill-the-box button) sit
    // together rather than the switch stretching to fill. The switch keeps its `AimToggleButton`
    // marker so the press → intent + sim-sync systems drive it parent-agnostically.
    let aim_panel = spawn_frame(
        commands,
        theme,
        AimPanel,
        Val::Percent(100.0),
        Val::Percent(BOTTOM_CELL_PCT),
    );
    commands
        .entity(aim_panel)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.flex_direction = FlexDirection::Row;
            n.justify_content = JustifyContent::Center;
            n.align_items = AlignItems::Center;
            n.column_gap = Val::Vw(GAP_VW);
        });
    let aim_label = spawn_aim_label(commands, theme);
    let aim_button = spawn_aim_button(commands, theme);
    commands
        .entity(aim_panel)
        .add_children(&[aim_label, aim_button]);
    // GTW-322 — a plain layout `Node` column (no markers); composed via `template_value`.
    let column_node = Node {
        width: Val::Percent(RIGHT_COL_PCT),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    };
    let column = commands.spawn_scene(template_value(column_node)).id();
    commands.entity(column).add_children(&[items, aim_panel]);
    column
}
