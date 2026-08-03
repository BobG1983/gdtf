use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::TextColor as UiTextColor,
    ui::{
        AlignItems, BackgroundColor, BorderColor, BorderRadius, FlexDirection, JustifyContent,
        Node, Overflow, UiRect, Val, widget::Button,
    },
};

use super::{
    style::segment_font,
    types::{
        ActiveSegment, Segment, SegmentColors, SegmentIndex, SegmentLabel, SegmentText,
        SegmentedControl,
    },
};
use crate::widgets::core::Orientation;

pub fn spawn_segmented_control(
    commands: &mut Commands,
    labels: &[SegmentLabel],
    active: usize,
    colors: SegmentColors,
    orientation: Orientation,
    marker: impl Bundle,
) -> Entity {
    let active = clamp_active(active, labels.len());
    let root_node = Node {
        flex_direction: orientation.flex_direction(),
        column_gap: Val::ZERO,
        row_gap: Val::ZERO,
        border_radius: BorderRadius::all(Val::Vw(SEGMENT_RADIUS_VW)),
        overflow: Overflow::clip(),
        ..default()
    };
    let segments: Vec<_> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let is_active = index == active;
            let seg_node = Node {
                padding: UiRect::axes(Val::Vw(SEGMENT_PAD_X_VW), Val::Vh(SEGMENT_PAD_Y_VH)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: segment_divider(orientation, index),
                ..default()
            };
            let seg_border = BorderColor::all(colors.base_text);
            let seg_bg = BackgroundColor(if is_active {
                colors.active_bg
            } else {
                colors.base_bg
            });
            let label_text_color = if is_active {
                colors.active_text
            } else {
                colors.base_text
            };
            let label_font = segment_font(is_active);
            let caption = label.to_string();
            (
                bsn! {
                    Segment
                    SegmentIndex::new(index)
                    Button
                    Children [
                        (
                            SegmentText
                            Text::new(caption)
                            UiTextColor(label_text_color)
                            template(move |_| Ok(label_font.clone()))
                        )
                    ]
                },
                template_value(seg_node),
                template_value(seg_border),
                template_value(seg_bg),
            )
        })
        .collect();
    commands
        .spawn_scene((
            bsn! {
                SegmentedControl
                ActiveSegment::new(active)
                Children [ { segments } ]
            },
            template_value(colors),
            template_value(root_node),
        ))
        .insert(marker)
        .id()
}

const fn segment_divider(orientation: Orientation, index: usize) -> UiRect {
    if index == 0 {
        return UiRect::ZERO;
    }
    let line = Val::Vw(SEGMENT_DIVIDER_VW);
    match orientation {
        Orientation::Horizontal => UiRect::left(line),
        Orientation::Vertical => UiRect::top(line),
    }
}

const fn clamp_active(active: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else if active >= count {
        count - 1
    } else {
        active
    }
}

const SEGMENT_PAD_X_VW: f32 = 0.9375;

const SEGMENT_PAD_Y_VH: f32 = 0.83333;

const SEGMENT_RADIUS_VW: f32 = 0.3125;

const SEGMENT_DIVIDER_VW: f32 = 0.078_125;
