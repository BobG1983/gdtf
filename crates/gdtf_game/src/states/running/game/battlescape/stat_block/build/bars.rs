use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::block::ROW_GAP_VH;

const BAR_LABEL_FONT_PT: f32 = 12.0;

pub(super) fn spawn_bar_label(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
) -> Entity {
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(BAR_LABEL_FONT_PT),
        ..default()
    };
    let node = Node {
        width: Val::Percent(100.0),
        ..default()
    };
    commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Text)
                Text::new("")
                UiTextColor(text_color)
                template(move |_| Ok(text_font.clone()))
            },
            template_value(TextLayout::justify(Justify::Right)),
            template_value(node),
        ))
        .insert(marker)
        .id()
}

pub(super) fn spawn_bar_group(commands: &mut Commands, label: Entity, bar: Entity) -> Entity {
    let node = Node {
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        row_gap: Val::Vh(ROW_GAP_VH),
        ..default()
    };
    let group = commands.spawn_scene(template_value(node)).id();
    commands.entity(group).add_children(&[label, bar]);
    group
}
