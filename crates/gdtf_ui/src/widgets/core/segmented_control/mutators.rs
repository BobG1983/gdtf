//! Mutate segment visibility and sub-line text.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{EntityCommandsSceneExt, bsn_list},
    text::TextColor as UiTextColor,
    ui::{Display, Node},
};

use super::{
    style::{sub_line_color, sub_line_font},
    types::{Segment, SegmentColors, SegmentIndex, SegmentSubLabel, SegmentSubText},
};

/// Show or hide the segment at `index` under `control`. Returns whether it was found.
pub fn set_segment_visible(
    control: Entity,
    index: usize,
    visible: bool,
    children: &Query<&Children>,
    segments: &mut Query<(&SegmentIndex, &mut Node), With<Segment>>,
) -> bool {
    let Ok(kids) = children.get(control) else {
        return false;
    };
    let want = if visible {
        Display::Flex
    } else {
        Display::None
    };
    for &child in kids {
        let Ok((seg_index, mut node)) = segments.get_mut(child) else {
            continue;
        };
        if **seg_index == index {
            if node.display != want {
                node.display = want;
            }
            return true;
        }
    }
    false
}

type SubLineSegment = (&'static SegmentIndex, &'static Children);

/// Set, replace, or clear the sub-line under a segment. Returns whether the segment was found.
pub fn set_segment_sub_line(
    commands: &mut Commands,
    control: Entity,
    index: usize,
    sub_line: Option<&SegmentSubLabel>,
    controls: &Query<(&Children, &SegmentColors)>,
    segments: &Query<SubLineSegment, With<Segment>>,
    sub_texts: &mut Query<&mut Text, With<SegmentSubText>>,
) -> bool {
    let Ok((kids, colors)) = controls.get(control) else {
        return false;
    };
    let dim = sub_line_color(colors.base_text);
    for &child in kids {
        let Ok((seg_index, seg_children)) = segments.get(child) else {
            continue;
        };
        if **seg_index != index {
            continue;
        }
        let existing = seg_children.iter().find(|&kid| sub_texts.get(kid).is_ok());
        match (sub_line, existing) {
            (Some(label), Some(node)) => {
                if let Ok(mut text) = sub_texts.get_mut(node) {
                    let want = label.to_string();
                    if text.0 != want {
                        text.0 = want;
                    }
                }
            }
            (Some(label), None) => {
                let caption = label.to_string();
                let font = sub_line_font();
                commands
                    .entity(child)
                    .queue_spawn_related_scenes::<Children>(bsn_list! {
                        (
                            SegmentSubText
                            Text::new(caption)
                            UiTextColor(dim)
                            template(move |_| Ok(font.clone()))
                        )
                    });
            }
            (None, Some(node)) => {
                commands.entity(node).despawn();
            }
            (None, None) => {}
        }
        return true;
    }
    false
}
