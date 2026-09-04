use bevy::prelude::*;
use gdtf_battle_sim::weapon::ModeKind;
use gdtf_ui::{
    Orientation, Segment, SegmentColors, SegmentIndex, SegmentLabel, spawn_segmented_control,
    theme::GdtfTheme,
};

use super::{
    super::stance_panel::control_segment_colors,
    order::{MODE_ORDER, mode_for_index, mode_index, mode_label},
};
use crate::states::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
};

pub(in crate::states::running::game::battlescape) fn spawn_mode_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let panel = gdtf_ui::spawn_panel(commands, theme);
    commands.entity(panel).insert((
        ModePanelRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            overflow: bevy::ui::Overflow {
                x: bevy::ui::OverflowAxis::Hidden,
                y: bevy::ui::OverflowAxis::Hidden,
            },
            ..default()
        },
        Visibility::Hidden,
    ));

    let labels: Vec<SegmentLabel> = MODE_ORDER
        .iter()
        .map(|k| SegmentLabel::new(mode_label(*k)))
        .collect();
    let control = spawn_segmented_control(
        commands,
        &labels,
        mode_index(ModeKind::Single),
        mode_segment_colors(theme),
        Orientation::Horizontal,
        ModeControl,
    );
    commands
        .entity(control)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        });
    commands.entity(panel).add_children(&[control]);
    panel
}

fn mode_segment_colors(theme: &GdtfTheme) -> SegmentColors {
    control_segment_colors(theme)
}

type NewModeSegment = (Entity, &'static SegmentIndex, &'static mut Node);

pub(in crate::states::running::game::battlescape) fn tag_mode_segments(
    mut commands: Commands,
    controls: Query<&Children, (With<ModeControl>, Added<ModeControl>)>,
    mut segments: Query<NewModeSegment, With<Segment>>,
) {
    for children in &controls {
        for &child in children {
            let Ok((segment, index, mut node)) = segments.get_mut(child) else {
                continue;
            };
            node.flex_grow = 1.0;
            node.flex_basis = Val::ZERO;
            node.min_width = Val::ZERO;
            node.min_height = Val::ZERO;
            node.overflow = bevy::ui::Overflow {
                x: bevy::ui::OverflowAxis::Hidden,
                y: bevy::ui::OverflowAxis::Hidden,
            };
            match mode_for_index(**index) {
                Some(ModeKind::Single) => {
                    commands.entity(segment).insert(ModeSingleButton);
                }
                Some(ModeKind::Burst) => {
                    commands.entity(segment).insert(ModeBurstButton);
                }
                Some(ModeKind::Full) => {
                    commands.entity(segment).insert(ModeFullButton);
                }
                None => {}
            }
        }
    }
}
