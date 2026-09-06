//! authoritative layout doc lives on the parent `spawn` module.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, LineBreak, TextColor as UiTextColor, TextFont, TextLayout},
    ui::{Display, Node, Overflow, OverflowAxis, UiRect, Val},
};
use gdtf_battle_input::PanelNavOrder;
use gdtf_ui::{
    ButtonLabel, spawn_button,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::geometry::{CONTENT_MIN_H_VH, GAP_VH};
use crate::states::running::game::battlescape::{
    focus_nav::WEAPON_NAV_BASE,
    weapon_panel::components::{
        CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage, WeaponMagazineText,
        WeaponNameText,
    },
};

fn spawn_image(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let slot_node = Node {
        width: Val::Percent(100.0),
        // The slot takes the height the name, magazine and Reload rows leave over.
        flex_grow: 1.0,
        flex_basis: Val::ZERO,
        min_height: Val::ZERO,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    let slot = commands
        .spawn_scene((
            bsn! {
                WeaponImage
                Themed::new(ThemeRole::Panel)
            },
            template_value(slot_node),
            template_value(background),
            template_value(border),
        ))
        .id();
    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    let label = commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new("no image")
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .id();
    commands.entity(slot).add_children(&[label]);
    slot
}

fn spawn_text(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    let text_color = *theme.text.text_color;
    let caption = initial.to_owned();
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new(caption)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert(marker)
        .id()
}

fn spawn_info_block(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let name = spawn_text(commands, theme, WeaponNameText, "—");
    commands
        .entity(name)
        .insert(TextLayout::linebreak(LineBreak::WordOrCharacter));
    let magazine = spawn_text(commands, theme, WeaponMagazineText, "0/0");
    let content_node = Node {
        width: Val::Percent(100.0),
        min_height: Val::Vh(CONTENT_MIN_H_VH),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        display: Display::None,
        ..default()
    };
    let content = commands
        .spawn_scene((
            bsn! { WeaponContent },
            template_value(content_node),
            template_value(Visibility::Hidden),
        ))
        .id();
    commands.entity(content).add_children(&[name, magazine]);

    let reload = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Reload"),
        (ReloadButton, PanelNavOrder::new(WEAPON_NAV_BASE)),
    );
    let reload_row_node = Node {
        width: Val::Percent(100.0),
        // The row is as tall as the Reload button and never gives that height away.
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        align_items: AlignItems::Center,
        ..default()
    };
    let reload_row = commands.spawn_scene(template_value(reload_row_node)).id();
    commands.entity(reload_row).add_children(&[reload]);

    let info_block_node = Node {
        width: Val::Percent(100.0),
        // The block is as tall as its text and its Reload row, and never gives that height away.
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        ..default()
    };
    let info_block = commands.spawn_scene(template_value(info_block_node)).id();
    commands
        .entity(info_block)
        .add_children(&[content, reload_row]);
    info_block
}

pub(super) fn spawn_combined_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    width: Val,
    height: Val,
) -> Entity {
    let image = spawn_image(commands, theme);
    let info_block = spawn_info_block(commands, theme);
    let panel_node = Node {
        width,
        height,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        overflow: Overflow {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        },
        border: UiRect::all(Val::Vw(*theme.panel.border_width)),
        ..default()
    };
    let background = BackgroundColor(*theme.panel.color);
    let border = bevy::ui::BorderColor::all(*theme.panel.border_color);
    let panel = commands
        .spawn_scene((
            bsn! {
                CombinedWeaponPanel
                Themed::new(ThemeRole::Panel)
            },
            template_value(panel_node),
            template_value(background),
            template_value(border),
        ))
        .id();
    commands.entity(panel).add_children(&[image, info_block]);
    panel
}
