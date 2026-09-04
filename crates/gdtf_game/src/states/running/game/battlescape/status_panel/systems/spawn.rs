use bevy::{
    prelude::*,
    ui::{Node, OverflowAxis, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    stat_block::spawn_stat_block,
    status_panel::{
        components::{StatusPanelRoot, StatusStatBlock},
        stability_readout::spawn_stability_readout,
    },
};

const PANEL_WIDTH_VW: f32 = 18.0;

const PANEL_MAX_HEIGHT_VH: f32 = 45.0;

pub(in crate::states::running::game::battlescape) fn spawn_status_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    atlases: Option<Res<TopDownAtlases>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        StatusPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Vh(0.0),
            left: Val::Vw(0.0),
            width: Val::Vw(PANEL_WIDTH_VW),
            height: Val::Auto,
            max_height: Val::Vh(PANEL_MAX_HEIGHT_VH),
            flex_direction: FlexDirection::Column,
            overflow: Overflow {
                x: OverflowAxis::Hidden,
                y: OverflowAxis::Hidden,
            },
            ..default()
        },
    ));

    let block = spawn_stat_block(&mut commands, &theme, atlases.as_deref());
    commands.entity(block).insert(StatusStatBlock);
    commands.entity(root).add_children(&[block]);

    let stability = spawn_stability_readout(&mut commands, &theme);
    commands.entity(root).add_children(&[stability]);
}

pub(in crate::states::running::game::battlescape) fn despawn_status_panel(
    mut commands: Commands,
    panels: Query<Entity, With<StatusPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
