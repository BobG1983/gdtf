//! (GTW-583); the authoritative layout doc lives on the parent `spawn` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, DisabledButton, spawn_button,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::geometry::GAP_VH;
use crate::states::running::game::battlescape::weapon_panel::components::{
    AimLabel, WeaponItemButton, WeaponItemPanel,
};

pub(super) fn spawn_frame(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    width: Val,
    height: Val,
) -> Entity {
    let node = Node {
        width,
        height,
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Panel) },
            template_value(node),
            template_value(background),
            template_value(border),
        ))
        .insert(marker)
        .id()
}

pub(super) fn spawn_aim_label(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            AimLabel
            Themed::new(ThemeRole::Text)
            Text::new("Aim")
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id()
}

pub(super) fn spawn_item_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    width: Val,
    height: Val,
) -> Entity {
    let panel = spawn_frame(commands, theme, WeaponItemPanel, width, height);
    let item_a = spawn_item_button(commands, theme);
    let item_b = spawn_item_button(commands, theme);
    commands.entity(panel).insert(Node {
        width,
        height,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    });
    commands.entity(panel).add_children(&[item_a, item_b]);
    panel
}

fn spawn_item_button(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let button = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Item"),
        (WeaponItemButton, DisabledButton),
    );
    commands.entity(button).insert(Node {
        width: Val::Percent(100.0),
        flex_grow: 1.0,
        flex_basis: Val::Percent(0.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    button
}
