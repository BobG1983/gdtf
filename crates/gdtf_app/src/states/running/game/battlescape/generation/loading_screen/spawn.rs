use bevy::{
    color::Alpha,
    prelude::*,
    state::prelude::DespawnOnExit,
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{
        AlignItems, BackgroundColor, GlobalZIndex, JustifyContent, Node, PositionType, UiRect, Val,
    },
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::{
        LOADING_BOX_MIN_H_VH, LOADING_BOX_W_VW, LOADING_SCREEN_Z, LoadingScreenRoot,
    },
};

const LOADING_CAPTION: &str = "GENERATING BATTLEFIELD…";

pub(in crate::states::running::game::battlescape::generation) fn spawn_loading_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let mut backdrop = *theme.background.color;
    backdrop.set_alpha(1.0);

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                width: Val::Vw(100.0),
                height: Val::Vh(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(backdrop),
            GlobalZIndex(LOADING_SCREEN_Z),
            LoadingScreenRoot,
            DespawnOnExit(BattleScapeState::Generation),
        ))
        .id();

    let box_entity = spawn_panel(&mut commands, &theme);
    commands.entity(box_entity).insert(Node {
        width: Val::Vw(LOADING_BOX_W_VW),
        min_height: Val::Vh(LOADING_BOX_MIN_H_VH),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: UiRect::all(Val::Vw(*theme.panel.border_width * 2.0)),
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    });
    commands.entity(root).add_child(box_entity);

    let title_font = TextFont {
        font: theme.title.font.clone().into(),
        font_size: FontSize::Px(*theme.title.font_size_pt),
        ..default()
    };
    let label = commands
        .spawn((
            Text::new(LOADING_CAPTION),
            UiTextColor(*theme.title.text_color),
            title_font,
        ))
        .id();
    commands.entity(box_entity).add_child(label);
}
