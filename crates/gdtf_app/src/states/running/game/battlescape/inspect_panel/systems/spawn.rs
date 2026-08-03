use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Display, Node, OverflowAxis, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{FillFraction, spawn_panel, spawn_progress_bar, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    inspect_panel::components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectIntegrity, InspectObjectProtection, InspectObjectText, InspectPanelRoot,
        InspectStatBlockHost,
    },
    stat_block::spawn_stat_block,
};

const INTEGRITY_REMAINING: Color = Color::srgb(0.55, 0.65, 0.45);

const INTEGRITY_LOST: Color = Color::srgb(0.12, 0.14, 0.16);

const ROW_GAP_VH: f32 = 0.55556;

const PANEL_WIDTH_VW: f32 = 18.0;

const PANEL_MAX_HEIGHT_VH: f32 = 45.0;

pub(in crate::states::running::game::battlescape) fn spawn_inspect_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    atlases: Option<Res<TopDownAtlases>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        InspectPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Vh(0.0),
            right: Val::Vw(0.0),
            width: Val::Vw(PANEL_WIDTH_VW),
            height: Val::Auto,
            max_height: Val::Vh(PANEL_MAX_HEIGHT_VH),
            flex_direction: FlexDirection::Column,
            overflow: Overflow {
                x: OverflowAxis::Hidden,
                y: OverflowAxis::Hidden,
            },
            ..default()
        },
        Visibility::Hidden,
    ));

    let block = spawn_stat_block(&mut commands, &theme, atlases.as_deref());
    commands.entity(block).insert(InspectStatBlockHost);

    let object_block = spawn_object_block(&mut commands, &theme);

    commands.entity(root).add_children(&[block, object_block]);
}

fn spawn_object_block(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let title = spawn_line(commands, theme, InspectObjectText, "");
    let integrity_label = spawn_line(commands, theme, InspectObjectIntegrity, "Integrity");
    let bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        INTEGRITY_REMAINING,
        INTEGRITY_LOST,
        InspectObjectBar,
    );
    let hardness = spawn_line(commands, theme, InspectObjectHardness, "");
    let protection = spawn_line(commands, theme, InspectObjectProtection, "");
    let height = spawn_line(commands, theme, InspectObjectHeight, "");

    let block_node = Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(ROW_GAP_VH),
        display: Display::None,
        ..default()
    };
    let block = commands
        .spawn_scene((bsn! { InspectObjectBlock }, template_value(block_node)))
        .id();
    commands.entity(block).add_children(&[
        title,
        integrity_label,
        bar,
        hardness,
        protection,
        height,
    ]);
    block
}

fn spawn_line(
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
            Text::new(caption)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert(marker)
        .id()
}

pub(in crate::states::running::game::battlescape) fn despawn_inspect_panel(
    mut commands: Commands,
    panels: Query<Entity, With<InspectPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
