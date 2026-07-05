//! Construction + the connected-look geometry of a [`SegmentedControl`]: the
//! [`spawn_segmented_control`] builder, the leading-edge divider, the active-index
//! clamp, and the layout constants.

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

/// Spawns a [`SegmentedControl`] with one segment per `labels` entry and returns the
/// ROOT [`Entity`].
///
/// `active` is the initially-active segment index (clamped to the last segment if out
/// of range); `colors` are the active/base background + text colors; `orientation`
/// lays the segments out as a Row or Column; `marker` is any [`Bundle`] the caller
/// wants on the root — typically its own identity marker so the
/// [`SegmentSelected`](super::SegmentSelected) listener can map it to an action.
///
/// Each segment is a [`Button`] (so `ui_focus_system` drives its
/// [`Interaction`](bevy::ui::Interaction), bevy-traps) with its label as a child
/// [`Text`](bevy::prelude::Text) INSIDE it — the label is the hit target. The active
/// segment is given the active fill + bold text at spawn;
/// [`repaint_segments`](super::repaint_segments)
/// re-derives the look on every active-index change ([[ui-mutate-not-respawn]]).
///
/// ## Connected look (GTW-277 screenshot review V2)
///
/// The control reads as ONE connected container, NOT a stack of detached pills: the
/// segments butt directly together (NO inter-segment gap), the ROOT carries the rounded
/// corners and CLIPS its children ([`Overflow::clip`](bevy::ui::Overflow::clip) + a
/// [`BorderRadius`] on the root) so the inner segment corners stay square and the whole
/// control reads as a single rounded box, and each segment after the first carries a thin
/// DIVIDER on its leading edge (left for a Row, top for a Column — the
/// [`base_text`](SegmentColors::base_text) color) so adjacent segments are visually
/// separated by a hairline rather than a gap. This is the "single container, adjacent
/// dividers, no gaps" the mockup's STAND/KNEEL/PRONE + SINGLE/BURST/AUTO controls show.
///
/// An empty `labels` yields a control with no segments (a no-op control).
pub fn spawn_segmented_control(
    commands: &mut Commands,
    labels: &[SegmentLabel],
    active: usize,
    colors: SegmentColors,
    orientation: Orientation,
    marker: impl Bundle,
) -> Entity {
    let active = clamp_active(active, labels.len());
    // The root container + per-segment layout nodes and the runtime active index / colors /
    // border color are bridged with `template_value` (no `bsn!` value-grammar form). The
    // segment COUNT is runtime (`labels.len()`), so the segment child scenes are built as a
    // runtime `Vec<Scene>` (a `SceneList`) and spliced into the relationship list via the
    // `{ expr }` scene-list-include grammar. Each segment's label `TextFont` is not `Unpin`
    // (so it rides neither `template_value` nor a field patch) — the `template(|_| ..)`
    // closure entry carries it. The caller's generic `marker` is `.insert`ed after (GTW-322).
    let root_node = Node {
        flex_direction: orientation.flex_direction(),
        // No inter-segment gap: a connected control's segments butt together, with
        // a hairline divider between them (set per-segment below) — not a gap (V2).
        column_gap: Val::ZERO,
        row_gap: Val::ZERO,
        // Round the OUTER container and clip the children, so the inner (square)
        // segment corners are hidden and the whole control reads as one rounded box.
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
                // A centered COLUMN so the OPTIONAL sub-line (GTW-303) stacks BELOW the
                // label. With a single centered child (the no-sub-line case) a Column with
                // both axes centered renders identically to a Row, so the stance control +
                // any pre-GTW-303 caller are visually unchanged; the label stays the
                // segment's first DIRECT child (so callers that post-process the label —
                // e.g. the action bar's nowrap pass — still reach it).
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                // No per-segment corner rounding — the ROOT owns the rounded corners
                // (square inner corners read as one connected control, V2). Each
                // segment after the first carries a leading-edge divider border.
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

/// The DIVIDER border of segment `index` in a control of the given `orientation`: a
/// hairline on the segment's LEADING edge (left for a [`Row`](Orientation::Horizontal),
/// top for a [`Column`](Orientation::Vertical)) for every segment after the first, and
/// none for the first segment (its leading edge is the control's outer edge).
///
/// The leading-edge-only divider gives ONE hairline between each pair of adjacent segments
/// (segment N's leading border butts against segment N-1's trailing edge) — the "adjacent
/// dividers" of a connected segmented control (V2), with no doubled lines.
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

/// Clamps a requested active index to a valid segment slot.
///
/// For a non-empty control the index is clamped to `0..count`; for an empty control it
/// degrades to `0` (no segment exists to highlight, so the value is inert).
const fn clamp_active(active: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else if active >= count {
        count - 1
    } else {
        active
    }
}

/// Horizontal inner padding of a segment, in viewport-width units.
/// Calibrated 12px / 1280 * 100.
const SEGMENT_PAD_X_VW: f32 = 0.9375;

/// Vertical inner padding of a segment, in viewport-height units.
/// Calibrated 6px / 720 * 100.
const SEGMENT_PAD_Y_VH: f32 = 0.83333;

/// The corner radius of the control's OUTER container, in viewport-width units (one axis
/// for radii, matching the border-width axis convention). The root rounds + clips; the
/// segments themselves are square so the control reads as one connected box (V2).
/// Calibrated 4px / 1280 * 100.
const SEGMENT_RADIUS_VW: f32 = 0.3125;

/// The width of the hairline DIVIDER between two adjacent segments, in viewport-width
/// units (one axis for a thin line — a connected control's "adjacent dividers", V2).
/// Calibrated 1px / 1280 * 100 at the default 1280x720 window.
const SEGMENT_DIVIDER_VW: f32 = 0.078_125;
