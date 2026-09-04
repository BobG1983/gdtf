use bevy::{
    prelude::*,
    ui::{FlexDirection, GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{ButtonLabel, spawn_button, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::{BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_X_VW, BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    select_cycle::components::{SelectCycleRoot, SelectNextButton, SelectPrevButton},
};

const CLUSTER_W_VW: f32 = 10.0;

const CLUSTER_Z: i32 = 11;

const BUTTON_H_PCT: f32 = 50.0;

const fn cluster_height_vh() -> f32 {
    BOTTOM_BAR_H_VH - 2.0 * BOTTOM_BAR_PAD_Y_VH
}

fn spawn_cycle_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
    label: &str,
    marker: impl Bundle,
) -> Entity {
    let button = spawn_button(commands, theme, ButtonLabel::new(label), marker);
    commands.entity(button).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(BUTTON_H_PCT),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    button
}

pub(in crate::states::running::game::battlescape) fn spawn_select_cycle(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    bottom_bar: Query<Entity, With<BottomBarRoot>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let next = spawn_cycle_button(&mut commands, &theme, "^ Next", SelectNextButton);
    let prev = spawn_cycle_button(&mut commands, &theme, "v Prev", SelectPrevButton);

    let root = commands
        .spawn((
            SelectCycleRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
                right: Val::Vw(BOTTOM_BAR_PAD_X_VW),
                width: Val::Vw(CLUSTER_W_VW),
                height: Val::Vh(cluster_height_vh()),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            GlobalZIndex(CLUSTER_Z),
        ))
        .id();
    commands.entity(root).add_children(&[next, prev]);

    if let Some(bar) = bottom_bar.iter().next() {
        commands.entity(bar).add_children(&[root]);
    }
}

pub(in crate::states::running::game::battlescape) fn despawn_select_cycle(
    mut commands: Commands,
    clusters: Query<Entity, With<SelectCycleRoot>>,
) {
    for cluster in &clusters {
        commands.entity(cluster).despawn();
    }
}
