use bevy::{
    color::Color,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_ui::{
    FillFraction, spawn_progress_bar,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::running::game::battlescape::status_panel::stability_readout::components::StabilityBar;

const STAB_CAPTION: &str = "STAB";

const STAB_REMAINING: Color = Color::srgb(0.30, 0.78, 0.36);

const STAB_LOST: Color = Color::srgb(0.12, 0.14, 0.16);

const STAB_CAPTION_FONT_PT: f32 = 12.0;

const ROW_GAP_VH: f32 = 0.55556;

pub(in crate::states::running::game::battlescape::status_panel) fn spawn_stability_readout(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let caption = spawn_caption(commands, theme);
    let bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        STAB_REMAINING,
        STAB_LOST,
        StabilityBar,
    );

    let row_node = Node {
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        row_gap: Val::Vh(ROW_GAP_VH),
        ..default()
    };
    let row = commands.spawn_scene(template_value(row_node)).id();
    commands.entity(row).add_children(&[caption, bar]);
    row
}

fn spawn_caption(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(STAB_CAPTION_FONT_PT),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new(STAB_CAPTION)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id()
}
