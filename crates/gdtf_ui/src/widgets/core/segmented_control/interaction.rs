//! Press-select + active-driven repaint: [`select_segment_on_press`] mutates the
//! root's [`ActiveSegment`] in place and emits [`SegmentSelected`];
//! [`repaint_segments`] re-styles every segment the same frame.

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

/// Read-write [`Query`] data for one pressed [`Segment`]: its [`SegmentIndex`] and its
/// [`ChildOf`](bevy::prelude::ChildOf) (to reach its control root).
///
/// Named to keep [`select_segment_on_press`]'s signature legible (clippy
/// `type_complexity`).
type PressedSegment = (
    &'static SegmentIndex,
    &'static ChildOf,
    &'static Interaction,
);

/// Sets a [`SegmentedControl`]'s active index when one of its segments is pressed,
/// MUTATING the root's [`ActiveSegment`] in place, then emits a [`SegmentSelected`]
/// message.
///
/// `Changed<Interaction>` + the explicit `== Pressed` test means one selection per
/// click (the press edge). Writing the root's [`ActiveSegment`] via
/// [`set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq) marks it changed only
/// on a REAL change, which is exactly the signal [`repaint_segments`] keys off — so
/// re-pressing the already-active segment neither repaints nor re-emits.
///
/// Param-only — no `&mut World` (bevy-traps rule 7). Registered by
/// [`UiPlugin`](crate::UiPlugin) in [`Update`] BEFORE [`repaint_segments`] so the
/// repaint sees the new active index the same frame; emits via
/// [`MessageWriter`](bevy::prelude::MessageWriter) (bevy-traps rule 4).
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

/// Read-only [`Query`] data for one segment during a repaint: its [`SegmentIndex`],
/// its [`BackgroundColor`](bevy::ui::BackgroundColor), and its [`Children`] (the label
/// text).
///
/// Named to keep [`repaint_segments`]'s signature legible (clippy `type_complexity`).
type RepaintSegment = (
    &'static SegmentIndex,
    &'static mut BackgroundColor,
    &'static Children,
);

/// Repaints ALL segments of every [`SegmentedControl`] whose [`ActiveSegment`] changed
/// this frame — the active gets the filled active background + bold active text, every
/// other returns to the base background + normal base text — in ONE pass, the SAME
/// frame the active index changed.
///
/// Active-driven, NOT hover-driven: it reacts to `Changed<ActiveSegment>` on the root
/// (set by [`select_segment_on_press`], or by a caller writing the component directly),
/// so the de-selected segment drops its active styling IMMEDIATELY — the GTW-280/284
/// stale-highlight lesson, applied generically in `gdtf_ui`. Both the background and
/// the label font weight + color are MUTATED in place ([[ui-mutate-not-respawn]]).
///
/// Param-only — no `&mut World` (bevy-traps rule 7). Registered by
/// [`UiPlugin`](crate::UiPlugin) in [`Update`]
/// `.after(`[`select_segment_on_press`]`)`.
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
            // Collect the label entities first: `seg_children` borrows the same query
            // we mutate through, so read the child ids out before the inner `get_mut`.
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
