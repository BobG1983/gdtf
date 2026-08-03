//! Select segments on press and repaint active colors.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, Interaction},
};

use super::{
    style::active_weight,
    types::{
        ActiveSegment, Segment, SegmentColors, SegmentIndex, SegmentSelected, SegmentText,
        SegmentedControl,
    },
};

type PressedSegment = (
    &'static SegmentIndex,
    &'static ChildOf,
    &'static Interaction,
);

/// On press, set the control's active segment and emit [`SegmentSelected`].
pub fn select_segment_on_press(
    segments: Query<PressedSegment, (Changed<Interaction>, With<Segment>)>,
    mut controls: Query<&mut ActiveSegment, With<SegmentedControl>>,
    mut selected: MessageWriter<SegmentSelected>,
) {
    for (index, parent, interaction) in &segments {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let control = parent.parent();
        let Ok(mut active) = controls.get_mut(control) else {
            continue;
        };
        if active.set_if_neq(ActiveSegment::new(**index)) {
            selected.write(SegmentSelected {
                control,
                index: *index,
            });
        }
    }
}

type RepaintSegment = (
    &'static SegmentIndex,
    &'static mut BackgroundColor,
    &'static Children,
);

/// When active segment changes, repaint segment backgrounds and text.
pub fn repaint_segments(
    controls: Query<(&ActiveSegment, &SegmentColors, &Children), Changed<ActiveSegment>>,
    mut segments: Query<RepaintSegment, With<Segment>>,
    mut texts: Query<(&mut TextFont, &mut UiTextColor), With<SegmentText>>,
) {
    for (active, colors, children) in &controls {
        for &child in children {
            let Ok((index, mut background, seg_children)) = segments.get_mut(child) else {
                continue;
            };
            let is_active = **index == **active;
            background.0 = if is_active {
                colors.active_bg
            } else {
                colors.base_bg
            };
            for &label in seg_children {
                if let Ok((mut font, mut color)) = texts.get_mut(label) {
                    font.weight = active_weight(is_active);
                    color.0 = if is_active {
                        colors.active_text
                    } else {
                        colors.base_text
                    };
                }
            }
        }
    }
}
