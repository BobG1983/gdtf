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

pub(super) fn spawn_left_column(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let combined = spawn_combined_panel(
        commands,
        theme,
        Val::Percent(100.0),
        Val::Percent(TOP_CELL_PCT),
    );
    let firemode = spawn_mode_panel(commands, theme);
    commands
        .entity(firemode)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.height = Val::Percent(BOTTOM_CELL_PCT);
        });
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

pub(super) fn spawn_right_column(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let items = spawn_item_panel(
        commands,
        theme,
        Val::Percent(100.0),
        Val::Percent(TOP_CELL_PCT),
    );
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
