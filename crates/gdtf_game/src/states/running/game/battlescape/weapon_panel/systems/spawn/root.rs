//! Weapon panel root spawn and teardown.
use bevy::{
    prelude::*,
    ui::{Node, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use super::{
    columns::{spawn_left_column, spawn_right_column},
    geometry::{GAP_VH, GAP_VW, PANEL_H_VH, PANEL_W_PCT, STANCE_W_PCT},
};
use crate::states::running::game::battlescape::{
    action_bar::spawn_stance_panel, bottom_bar::BottomBarRoot,
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
    let Some(bar) = bottom_bar.iter().next() else {
        return;
    };

    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        WeaponPanelRoot,
        Node {
            width: Val::Percent(PANEL_W_PCT),
            height: Val::Vh(PANEL_H_VH),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Vw(GAP_VW),
            ..default()
        },
    ));

    let left = spawn_left_column(&mut commands, &theme);
    let right = spawn_right_column(&mut commands, &theme);

    commands.entity(root).add_children(&[left, right]);

    let stance = spawn_stance_panel(&mut commands, &theme);
    commands.entity(stance).insert(Node {
        width: Val::Percent(STANCE_W_PCT),
        height: Val::Vh(PANEL_H_VH),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        // The row's free space collects here, so the panels after it sit at the bar's right end.
        margin: UiRect::right(Val::Auto),
        ..default()
    });
    commands.entity(bar).add_children(&[root, stance]);
}

pub(in crate::states::running::game::battlescape) fn despawn_weapon_panel(
    mut commands: Commands,
    panels: Query<Entity, With<WeaponPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
