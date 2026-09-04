use bevy::{
    color::Alpha,
    prelude::*,
    ui::{BackgroundColor, FlexDirection, GlobalZIndex, Node, Overflow, PositionType, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::BOTTOM_BAR_H_VH,
    combat_log::{
        components::{CombatLogRoot, PanelHeightAnim},
        tuning::CombatLogTuning,
    },
};

const COMBAT_LOG_Z: i32 = 11;

const COMBAT_LOG_BG_ALPHA: f32 = 0.45;

pub(in crate::states::running::game::battlescape) fn spawn_combat_log(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    tuning: Option<Res<CombatLogTuning>>,
) {
    let Some(theme) = theme else {
        return;
    };
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);

    let root = spawn_panel(&mut commands, &theme);
    let mut fill = *theme.panel.color;
    fill.set_alpha(COMBAT_LOG_BG_ALPHA);
    commands.entity(root).insert((
        CombatLogRoot,
        PanelHeightAnim::new(0.0),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Vh(BOTTOM_BAR_H_VH),
            left: Val::ZERO,
            width: Val::Vw(*tuning.panel_width_vw),
            height: Val::Px(0.0),
            overflow: Overflow::clip(),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(fill),
        GlobalZIndex(COMBAT_LOG_Z),
    ));
}

pub(in crate::states::running::game::battlescape) fn despawn_combat_log(
    mut commands: Commands,
    logs: Query<Entity, With<CombatLogRoot>>,
) {
    for log in &logs {
        commands.entity(log).despawn();
    }
}
