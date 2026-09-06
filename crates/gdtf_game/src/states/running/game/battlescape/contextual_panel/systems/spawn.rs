use bevy::{
    prelude::*,
    ui::{Node, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::BottomBarRoot,
    contextual_panel::components::{
        CONTEXTUAL_PANEL_ROW_GAP_VH, CONTEXTUAL_PANEL_WIDTH_PCT, ContextualPanelRoot,
    },
};

pub(in crate::states::running::game::battlescape) fn spawn_contextual_panel(
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

    let panel = spawn_panel(&mut commands, &theme);
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = Val::Percent(CONTEXTUAL_PANEL_WIDTH_PCT);
            node.height = Val::Auto;
            node.align_self = AlignSelf::FlexEnd;
            node.flex_direction = FlexDirection::Column;
            node.row_gap = Val::Vh(CONTEXTUAL_PANEL_ROW_GAP_VH);
        });
    commands
        .entity(panel)
        .insert((ContextualPanelRoot, Visibility::Hidden));
    commands.entity(bar).add_children(&[panel]);
}

pub(in crate::states::running::game::battlescape) fn despawn_contextual_panel(
    mut commands: Commands,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
