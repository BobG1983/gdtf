use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_battle_input::PanelNavOrder;
use gdtf_ui::{ButtonLabel, spawn_button, spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    action_bar::components::{
        ActionBarRoot, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
    },
    focus_nav::ACTION_BAR_NAV_BASE,
};

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct BarGapVw(f32);

impl BarGapVw {
    const BAR: Self = Self(0.625);
}

pub(in crate::states::running::game::battlescape) fn spawn_action_bar(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let root_node = Node {
        position_type: PositionType::Absolute,
        top: Val::ZERO,
        left: Val::Vw(0.0),
        width: Val::Vw(100.0),
        height: Val::Auto,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::FlexStart,
        ..default()
    };
    let root = commands
        .spawn_scene((bsn! { ActionBarRoot }, template_value(root_node)))
        .id();

    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert(Node {
        width: Val::Auto,
        height: Val::Auto,
        column_gap: Val::Vw(*BarGapVw::BAR),
        ..default()
    });

    let level_up = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Level +"),
        (LevelUpButton, PanelNavOrder::new(ACTION_BAR_NAV_BASE)),
    );
    let level_down = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Level -"),
        (LevelDownButton, PanelNavOrder::new(ACTION_BAR_NAV_BASE + 1)),
    );

    let end_turn = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("End Turn"),
        (EndTurnButton, PanelNavOrder::new(ACTION_BAR_NAV_BASE + 2)),
    );

    let flee = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Flee"),
        (FleeButton, PanelNavOrder::new(ACTION_BAR_NAV_BASE + 3)),
    );

    commands
        .entity(panel)
        .add_children(&[level_up, level_down, end_turn, flee]);
    commands.entity(root).add_children(&[panel]);
}

pub(in crate::states::running::game::battlescape) fn despawn_action_bar(
    mut commands: Commands,
    bars: Query<Entity, With<ActionBarRoot>>,
) {
    for bar in &bars {
        commands.entity(bar).despawn();
    }
}
