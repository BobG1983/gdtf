//! Weapon panel root spawn and teardown.
use bevy::{
    prelude::*,
    ui::{GlobalZIndex, Node, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use super::{
    columns::{spawn_left_column, spawn_right_column},
    geometry::{GAP_VH, GAP_VW, PANEL_H_VH, PANEL_W_VW, PANEL_Z, STANCE_LEFT_VW, STANCE_W_VW},
};
use crate::states::running::game::battlescape::{
    action_bar::spawn_stance_panel,
    bottom_bar::{BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    weapon_panel::components::WeaponPanelRoot,
};

pub(in crate::states::running::game::battlescape) fn spawn_weapon_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    bottom_bar: Query<Entity, With<BottomBarRoot>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        WeaponPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
            left: Val::ZERO,
            width: Val::Vw(PANEL_W_VW),
            height: Val::Vh(PANEL_H_VH),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Vw(GAP_VW),
            ..default()
        },
        GlobalZIndex(PANEL_Z),
    ));

    let left = spawn_left_column(&mut commands, &theme);
    let right = spawn_right_column(&mut commands, &theme);

    commands.entity(root).add_children(&[left, right]);

    let stance = spawn_stance_panel(&mut commands, &theme);
    commands.entity(stance).insert(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
        left: Val::Vw(STANCE_LEFT_VW),
        width: Val::Vw(STANCE_W_VW),
        height: Val::Vh(PANEL_H_VH),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    });
    let stance_parent = bottom_bar.iter().next().unwrap_or(root);
    commands.entity(stance_parent).add_children(&[stance]);
}

pub(in crate::states::running::game::battlescape) fn despawn_weapon_panel(
    mut commands: Commands,
    panels: Query<Entity, With<WeaponPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
