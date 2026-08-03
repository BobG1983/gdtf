//! colors are **not** authoritative: the central
use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use super::markers::ButtonLabel;
use crate::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

pub fn spawn_panel(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let node = box_node(
        *theme.panel.border_width,
        *theme.panel.corner_radius,
        theme,
        BoxKind::Panel,
    );
    let border_color = UiBorderColor::all(*theme.panel.border_color);
    let background = BackgroundColor(*theme.panel.color);
    commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Panel) },
            template_value(node),
            template_value(background),
            template_value(border_color),
        ))
        .id()
}

pub fn spawn_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
    label: ButtonLabel,
    marker: impl Bundle,
) -> Entity {
    let font = theme.button.font.clone();
    let font_size = *theme.button.font_size_pt;
    let text_color = *theme.button.text_color;
    let node = box_node(
        *theme.button.border_width,
        *theme.button.corner_radius,
        theme,
        BoxKind::Button,
    );
    let border_color = UiBorderColor::all(*theme.button.border_color);
    let background = BackgroundColor(*theme.button.color);
    let text_font = TextFont {
        font: font.into(),
        font_size: FontSize::Px(font_size),
        ..default()
    };
    let caption = label.into_inner();

    commands
        .spawn_scene((
            bsn! {
                Button
                Themed::new(ThemeRole::Button)
                Children [
                    (
                        Themed::new(ThemeRole::ButtonText)
                        Text::new(caption)
                        UiTextColor(text_color)
                        template(move |_| Ok(text_font.clone()))
                    )
                ]
            },
            template_value(node),
            template_value(background),
            template_value(border_color),
        ))
        .insert(marker)
        .id()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BoxKind {
        Panel,
        Button,
}

fn box_node(border_vw: f32, radius_vw: f32, theme: &GdtfTheme, kind: BoxKind) -> Node {
    let margin = match kind {
        BoxKind::Panel => theme.panel.margin,
        BoxKind::Button => theme.button.margin,
    };
    Node {
        border: UiRect::all(Val::Vw(border_vw)),
        border_radius: BorderRadius::all(Val::Vw(radius_vw)),
        padding: UiRect {
            left:   Val::Vw(*margin.l),
            right:  Val::Vw(*margin.r),
            top:    Val::Vh(*margin.t),
            bottom: Val::Vh(*margin.b),
        },
        ..default()
    }
}

#[cfg(test)]
mod test;
