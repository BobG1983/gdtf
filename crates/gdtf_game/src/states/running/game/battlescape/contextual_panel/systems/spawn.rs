use bevy::{
    prelude::*,
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::{BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    contextual_panel::components::{
        CONTEXTUAL_PANEL_RIGHT_VW, CONTEXTUAL_PANEL_ROW_GAP_VH, CONTEXTUAL_PANEL_WIDTH_VW,
        CONTEXTUAL_PANEL_Z, ContextualPanelRoot,
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

    let panel = spawn_panel(&mut commands, &theme);
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.right = Val::Vw(CONTEXTUAL_PANEL_RIGHT_VW);
            node.bottom = Val::Vh(BOTTOM_BAR_PAD_Y_VH);
            node.width = Val::Vw(CONTEXTUAL_PANEL_WIDTH_VW);
            node.height = Val::Auto;
            node.flex_direction = FlexDirection::Column;
            node.row_gap = Val::Vh(CONTEXTUAL_PANEL_ROW_GAP_VH);
        });
    commands
        .entity(panel)
        .insert((ContextualPanelRoot, Visibility::Hidden));

    match bottom_bar.iter().next() {
        Some(bar) => {
            commands.entity(bar).add_children(&[panel]);
        }
        None => {
            commands
                .entity(panel)
                .insert(GlobalZIndex(CONTEXTUAL_PANEL_Z));
        }
    }
}

pub(in crate::states::running::game::battlescape) fn despawn_contextual_panel(
    mut commands: Commands,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
