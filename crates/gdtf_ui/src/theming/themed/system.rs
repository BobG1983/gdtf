//! Apply theme colors to entities marked with [`Themed`].

use bevy::{
    prelude::*,
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, BorderRadius, Node, UiRect, Val},
};

use super::role::{ThemeRole, Themed};
use crate::theme::{ContentMargin, GdtfTheme};

type ThemedData<'a> = (Entity, &'a Themed, Option<&'a Node>);

type AddedOrChangedThemed = Or<(Added<Themed>, Changed<Themed>)>;

/// Paint themed entities when the theme changes or new themed nodes appear.
pub fn apply_theme(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    full: Query<ThemedData>,
    incremental: Query<ThemedData, AddedOrChangedThemed>,
) {
    let mut painted = 0usize;
    if theme.is_changed() {
        for data in &full {
            paint_themed(&mut commands, &theme, data);
            painted += 1;
        }
    } else {
        for data in &incremental {
            paint_themed(&mut commands, &theme, data);
            painted += 1;
        }
    }
    if painted > 0 {
        info!("apply_theme: repainted {painted} Themed entities from the current GdtfTheme");
    }
}

fn paint_themed(commands: &mut Commands, theme: &GdtfTheme, (entity, marker, node): ThemedData) {
    match **marker {
        ThemeRole::Background => {
            commands
                .entity(entity)
                .insert(BackgroundColor(*theme.background.color));
        }
        ThemeRole::Panel => {
            let themed_node = box_node(
                node,
                *theme.panel.border_width,
                *theme.panel.corner_radius,
                theme.panel.margin,
            );
            commands.entity(entity).insert((
                BackgroundColor(*theme.panel.color),
                UiBorderColor::all(*theme.panel.border_color),
                themed_node,
            ));
        }
        ThemeRole::Button => {
            let themed_node = box_node(
                node,
                *theme.button.border_width,
                *theme.button.corner_radius,
                theme.button.margin,
            );
            commands.entity(entity).insert((
                BackgroundColor(*theme.button.color),
                UiBorderColor::all(*theme.button.border_color),
                themed_node,
            ));
        }
        ThemeRole::ButtonText => {
            commands.entity(entity).insert((
                UiTextColor(*theme.button.text_color),
                TextFont {
                    font: theme.button.font.clone().into(),
                    font_size: FontSize::Px(*theme.button.font_size_pt),
                    ..default()
                },
            ));
        }
        ThemeRole::Title => {
            commands.entity(entity).insert((
                UiTextColor(*theme.title.text_color),
                TextFont {
                    font: theme.title.font.clone().into(),
                    font_size: FontSize::Px(*theme.title.font_size_pt),
                    ..default()
                },
            ));
        }
        ThemeRole::Text => {
            commands.entity(entity).insert((
                UiTextColor(*theme.text.text_color),
                TextFont {
                    font: theme.text.font.clone().into(),
                    font_size: FontSize::Px(*theme.text.font_size_pt),
                    ..default()
                },
            ));
        }
    }
}

fn box_node(node: Option<&Node>, border_vw: f32, radius_vw: f32, margin: ContentMargin) -> Node {
    let mut themed_node = node.cloned().unwrap_or_default();
    themed_node.border = UiRect::all(Val::Vw(border_vw));
    themed_node.border_radius = BorderRadius::all(Val::Vw(radius_vw));
    themed_node.padding = UiRect {
        left: Val::Vw(*margin.l),
        right: Val::Vw(*margin.r),
        top: Val::Vh(*margin.t),
        bottom: Val::Vh(*margin.b),
    };
    themed_node
}

/// True when any entity gained a [`Themed`] component this frame.
#[must_use]
pub fn any_themed_added(added: Query<(), Added<Themed>>) -> bool {
    !added.is_empty()
}
