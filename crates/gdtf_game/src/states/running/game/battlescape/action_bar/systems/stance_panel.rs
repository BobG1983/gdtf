use bevy::{
    prelude::*,
    ui::{Node, UiRect, Val},
};
use gdtf_battle_sim::prelude::StanceKind;
use gdtf_ui::{
    Orientation, SegmentColors, SegmentLabel, spawn_panel, spawn_segmented_control,
    theme::GdtfTheme,
};

use crate::states::running::game::battlescape::action_bar::components::{
    StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton, StanceStandingButton,
};

const fn stance_label(kind: StanceKind) -> &'static str {
    match kind {
        StanceKind::Standing => "Stand",
        StanceKind::Crouching => "Kneel",
        StanceKind::Prone => "Prone",
    }
}

pub(in crate::states::running::game::battlescape) const STANCE_ORDER: [StanceKind; 3] = [
    StanceKind::Standing,
    StanceKind::Crouching,
    StanceKind::Prone,
];

pub(in crate::states::running::game::battlescape) fn stance_index(kind: StanceKind) -> usize {
    STANCE_ORDER.iter().position(|k| *k == kind).unwrap_or(0)
}

pub(in crate::states::running::game::battlescape) fn stance_for_index(
    index: usize,
) -> Option<StanceKind> {
    STANCE_ORDER.get(index).copied()
}

pub(in crate::states::running::game::battlescape) fn control_segment_colors(
    theme: &GdtfTheme,
) -> SegmentColors {
    SegmentColors {
        active_bg:   *theme.button.active,
        active_text: *theme.button.text_color,
        base_bg:     *theme.button.color,
        base_text:   *theme.button.text_color,
    }
}

pub(in crate::states::running::game::battlescape) fn spawn_stance_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let panel = spawn_panel(commands, theme);
    commands.entity(panel).insert((
        StancePanelRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Vw(*theme.panel.border_width)),
            ..default()
        },
    ));

    let labels: Vec<SegmentLabel> = STANCE_ORDER
        .iter()
        .map(|k| SegmentLabel::new(stance_label(*k)))
        .collect();
    let control = spawn_segmented_control(
        commands,
        &labels,
        stance_index(StanceKind::Standing),
        control_segment_colors(theme),
        Orientation::Vertical,
        StanceControl,
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

type NewStanceSegment = (Entity, &'static gdtf_ui::SegmentIndex, &'static mut Node);

pub(in crate::states::running::game::battlescape) fn tag_stance_segments(
    mut commands: Commands,
    controls: Query<&Children, (With<StanceControl>, Added<StanceControl>)>,
    mut segments: Query<NewStanceSegment, With<gdtf_ui::Segment>>,
) {
    for children in &controls {
        for &child in children {
            let Ok((segment, index, mut node)) = segments.get_mut(child) else {
                continue;
            };
            node.width = Val::Percent(100.0);
            node.flex_grow = 1.0;
            node.flex_basis = Val::ZERO;
            node.min_height = Val::ZERO;
            match stance_for_index(**index) {
                Some(StanceKind::Standing) => {
                    commands.entity(segment).insert(StanceStandingButton);
                }
                Some(StanceKind::Crouching) => {
                    commands.entity(segment).insert(StanceKneelingButton);
                }
                Some(StanceKind::Prone) => {
                    commands.entity(segment).insert(StanceProneButton);
                }
                None => {}
            }
        }
    }
}
