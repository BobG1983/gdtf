//! The [`SegmentedControl`] widget: N adjacent labeled segments, exactly one active.
//!
//! A segmented control is the Fire-Mode (Single/Burst/Full-Auto) and the stance
//! (Stand/Kneel/Prone) controls in the mockup: adjacent LABELED segments where
//! exactly one is active. The active segment reads as a FILLED background **plus
//! bold** text — NOT color alone, so it is color-blind-safe. The label sits INSIDE
//! each segment, so the label IS the hit target. Orientation is a flex-direction
//! param ([`Orientation::Horizontal`] = Row, [`Orientation::Vertical`] = Column).
//!
//! Changing the active segment repaints ALL segments the SAME frame
//! ([`repaint_segments`] reacts to the [`ActiveSegment`] index change on the root,
//! NOT to hover — the GTW-280/284 lesson: the de-selected segment returns to its base
//! look immediately). All repaints MUTATE existing segment entities in place; nothing
//! is despawned/respawned on a selection change ([[ui-mutate-not-respawn]]).
//!
//! Selecting a new segment emits a [`SegmentSelected`] message carrying the chosen
//! segment's IDENTITY (the control [`Entity`] + the segment index), so a downstream
//! listener maps it to an action; `gdtf_ui` defines the message, never the act.

use bevy::{
    prelude::*,
    text::{FontWeight, TextColor as UiTextColor, TextFont},
    ui::{
        AlignItems, BackgroundColor, BorderRadius, Interaction, JustifyContent, Node, UiRect, Val,
        widget::Button,
    },
};

use super::orientation::Orientation;

/// The caption of one [`SegmentedControl`] segment.
///
/// A named newtype over the label string (mirroring
/// [`ButtonLabel`](super::ButtonLabel), no-bare-types rule): a segment's label is a
/// widget-level value, not arbitrary text.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct SegmentLabel(String);

impl SegmentLabel {
    /// Wraps a caption into a [`SegmentLabel`].
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

/// The active/base color + font pair a [`SegmentedControl`] paints its segments with.
///
/// Pure UI plumbing ([`bevy::Color`]s): the active segment gets `active_bg` +
/// `active_text` (and **bold** weight), every other segment gets `base_bg` +
/// `base_text` (normal weight). The bold weight on the active segment is the
/// color-blind-safe second channel the contract requires (NOT color alone). Stored as
/// a [`Component`] on the control root so [`repaint_segments`] re-derives every
/// segment's look from the active index without the caller re-passing colors.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct SegmentColors {
    /// Background fill of the ACTIVE segment.
    pub active_bg:   Color,
    /// Text color of the ACTIVE segment.
    pub active_text: Color,
    /// Background fill of a NON-active segment.
    pub base_bg:     Color,
    /// Text color of a NON-active segment.
    pub base_text:   Color,
}

/// Marker on the ROOT container of a [`SegmentedControl`].
///
/// The root lays its segment children out along [`Orientation::flex_direction`]; each
/// child is a [`Segment`]. The caller attaches its own identity marker alongside this
/// so the [`SegmentSelected`] listener can map the control to an action.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentedControl;

/// The currently-active segment index, held on the [`SegmentedControl`] ROOT.
///
/// [`repaint_segments`] reacts to a CHANGE of this component (`Changed<ActiveSegment>`)
/// and repaints ALL the root's segments the same frame — active-driven, not hover
/// (the GTW-280/284 lesson). A named newtype over the index (no-bare-types rule).
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActiveSegment(usize);

/// Marker on one segment (a [`Button`] child of a [`SegmentedControl`] root).
///
/// Each segment carries its [`SegmentIndex`]; [`repaint_segments`] re-styles it by
/// comparing its index to the root's [`ActiveSegment`].
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Segment;

/// The position of a [`Segment`] within its [`SegmentedControl`].
///
/// A named newtype over the index (no-bare-types rule): a segment's slot is a
/// widget-level value used to compare against the root's [`ActiveSegment`] and to
/// report which segment was chosen in [`SegmentSelected`].
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentIndex(usize);

/// Marker on the LABEL text child of a [`Segment`].
///
/// [`repaint_segments`] toggles its [`TextFont`](bevy::text::TextFont) weight
/// (bold on the active segment, normal otherwise — the color-blind-safe channel) and
/// its [`TextColor`](bevy::text::TextColor).
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentText;

/// A buffered Bevy **message** emitted when a [`SegmentedControl`]'s active segment
/// changes (bevy-traps rule 4: buffered events are messages in 0.18).
///
/// Carries the control's [`Entity`] and the chosen [`SegmentIndex`] — the IDENTITY of
/// the selection. `gdtf_ui` cannot know what a segment MEANS (a fire mode, a stance),
/// so it reports only *which* control and *which* segment; a downstream listener reads
/// the caller-attached marker off the control entity and maps the index to an action.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SegmentSelected {
    /// The segmented-control root entity (carries the caller's identity marker).
    pub control: Entity,
    /// The index of the newly-active segment.
    pub index:   SegmentIndex,
}

/// Spawns a [`SegmentedControl`] with one segment per `labels` entry and returns the
/// ROOT [`Entity`].
///
/// `active` is the initially-active segment index (clamped to the last segment if out
/// of range); `colors` are the active/base background + text colors; `orientation`
/// lays the segments out as a Row or Column; `marker` is any [`Bundle`] the caller
/// wants on the root — typically its own identity marker so the [`SegmentSelected`]
/// listener can map it to an action.
///
/// Each segment is a [`Button`] (so `ui_focus_system` drives its
/// [`Interaction`](bevy::ui::Interaction), bevy-traps) with its label as a child
/// [`Text`](bevy::prelude::Text) INSIDE it — the label is the hit target. The active
/// segment is given the active fill + bold text at spawn; [`repaint_segments`]
/// re-derives the look on every active-index change ([[ui-mutate-not-respawn]]).
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
    commands
        .spawn((
            SegmentedControl,
            ActiveSegment(active),
            colors,
            Node {
                flex_direction: orientation.flex_direction(),
                column_gap: Val::Px(SEGMENT_GAP_PX),
                row_gap: Val::Px(SEGMENT_GAP_PX),
                ..default()
            },
            marker,
        ))
        .with_children(|root| {
            for (index, label) in labels.iter().enumerate() {
                let is_active = index == active;
                root.spawn((
                    Segment,
                    SegmentIndex(index),
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(SEGMENT_PAD_X_PX), Val::Px(SEGMENT_PAD_Y_PX)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(Val::Px(SEGMENT_RADIUS_PX)),
                        ..default()
                    },
                    BackgroundColor(if is_active {
                        colors.active_bg
                    } else {
                        colors.base_bg
                    }),
                ))
                .with_children(|seg| {
                    seg.spawn((
                        SegmentText,
                        Text::new(label.to_string()),
                        segment_font(is_active),
                        UiTextColor(if is_active {
                            colors.active_text
                        } else {
                            colors.base_text
                        }),
                    ));
                });
            }
        })
        .id()
}

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
        if active.set_if_neq(ActiveSegment(**index)) {
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

/// The [`FontWeight`](bevy::text::FontWeight) of a segment label by active-ness — the
/// color-blind-safe second channel (bold on active, normal otherwise).
const fn active_weight(is_active: bool) -> FontWeight {
    if is_active {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    }
}

/// The label [`TextFont`](bevy::text::TextFont) for a segment by active-ness.
fn segment_font(is_active: bool) -> TextFont {
    TextFont {
        font_size: SEGMENT_FONT_PT,
        weight: active_weight(is_active),
        ..default()
    }
}

/// The gap between adjacent segments, in logical pixels.
const SEGMENT_GAP_PX: f32 = 2.0;

/// Horizontal inner padding of a segment, in logical pixels.
const SEGMENT_PAD_X_PX: f32 = 12.0;

/// Vertical inner padding of a segment, in logical pixels.
const SEGMENT_PAD_Y_PX: f32 = 6.0;

/// The corner radius of a segment, in logical pixels.
const SEGMENT_RADIUS_PX: f32 = 4.0;

/// The segment label font size, in typographic points.
const SEGMENT_FONT_PT: f32 = 16.0;
