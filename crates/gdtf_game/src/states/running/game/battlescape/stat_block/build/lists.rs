use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::running::game::battlescape::stat_block::components::{
    StatInjuryLine, StatInjuryList, StatWoundLine, StatWoundList,
};

pub(super) const WOUND_LINE_POOL: usize = 8;

pub(super) const INJURY_LINE_POOL: usize = 8;

pub(super) fn spawn_wound_list(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let container_node = Node {
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let container = commands
        .spawn_scene((
            bsn! { StatWoundList },
            template_value(container_node),
            template_value(Visibility::Hidden),
        ))
        .id();
    let text_color = *theme.text.text_color;
    let lines: Vec<Entity> = (0..WOUND_LINE_POOL)
        .map(|_| {
            let text_font = TextFont {
                font: theme.text.font.clone().into(),
                font_size: FontSize::Px(*theme.text.font_size_pt),
                ..default()
            };
            commands
                .spawn_scene((
                    bsn! {
                        StatWoundLine
                        Themed::new(ThemeRole::Text)
                        Text::new("")
                        UiTextColor(text_color)
                        template(move |_| Ok(text_font.clone()))
                    },
                    template_value(Visibility::Hidden),
                ))
                .id()
        })
        .collect();
    commands.entity(container).add_children(&lines);
    container
}

pub(super) fn spawn_injury_list(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let container_node = Node {
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let container = commands
        .spawn_scene((
            bsn! { StatInjuryList },
            template_value(container_node),
            template_value(Visibility::Hidden),
        ))
        .id();
    let text_color = *theme.text.text_color;
    let lines: Vec<Entity> = (0..INJURY_LINE_POOL)
        .map(|_| {
            let text_font = TextFont {
                font: theme.text.font.clone().into(),
                font_size: FontSize::Px(*theme.text.font_size_pt),
                ..default()
            };
            commands
                .spawn_scene((
                    bsn! {
                        StatInjuryLine
                        Themed::new(ThemeRole::Text)
                        Text::new("")
                        UiTextColor(text_color)
                        template(move |_| Ok(text_font.clone()))
                    },
                    template_value(Visibility::Hidden),
                ))
                .id()
        })
        .collect();
    commands.entity(container).add_children(&lines);
    container
}
