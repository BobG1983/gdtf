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
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, EntityCommandsSceneExt, bsn, bsn_list, template_value},
    text::{FontSize, FontWeight, TextColor as UiTextColor, TextFont},
    ui::{
        AlignItems, BackgroundColor, BorderColor, BorderRadius, Display, FlexDirection,
        Interaction, JustifyContent, Node, Overflow, UiRect, Val, widget::Button,
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

/// The OPTIONAL secondary caption ("sub-line") under a [`SegmentedControl`] segment's
/// primary [`SegmentLabel`].
///
/// A named newtype over the sub-line string (mirroring [`SegmentLabel`], no-bare-types
/// rule): a segment's sub-line is a smaller, dimmer second line of widget-level text
/// (e.g. the firemode's TU cost under its name — GTW-303), rendered by a separate
/// [`SegmentSubText`] node so it can carry its own smaller font + dimmer color (a single
/// `Text` with an embedded newline cannot give two lines different styling). It is set /
/// cleared in place via [`set_segment_sub_line`].
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct SegmentSubLabel(String);

impl SegmentSubLabel {
    /// Wraps a sub-line caption into a [`SegmentSubLabel`].
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
///
/// The derived [`Default`] (all-black) is a **spawn-seed sentinel only** (GTW-322):
/// the `bsn!` scene path seeds the slot with [`Default`] before
/// [`template_value`](bevy::scene::template_value) overwrites it with the caller's
/// colors. It is never a meaningful palette — a builder always supplies real colors.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
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

impl ActiveSegment {
    /// Wraps a segment index into an [`ActiveSegment`].
    ///
    /// The constructor a caller uses to DRIVE the active segment from code (e.g. syncing
    /// the highlight to a sim value): write it onto the control root with
    /// [`set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq) and
    /// [`repaint_segments`] repaints the same frame. The index is clamped by
    /// [`repaint_segments`]' equality check (an out-of-range value simply highlights
    /// nothing), so no clamp is needed here.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

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

/// Marker on the OPTIONAL sub-line text child of a [`Segment`] (GTW-303).
///
/// A second [`Text`](bevy::prelude::Text) node STACKED below the segment's primary
/// [`SegmentText`] label (the segment [`Button`] is a centered [`Column`](FlexDirection::Column)),
/// rendered at a SMALLER font ([`SEGMENT_SUB_FONT_PT`]) and a DIMMER color ([`sub_line_color`])
/// than the label so it reads as a quiet secondary line. It is created / removed in place by
/// [`set_segment_sub_line`] — a segment with no sub-line has NO node carrying this marker, so it
/// renders exactly as a single centered label (the stance control + any pre-GTW-303 caller are
/// visually unchanged). UNLIKE [`SegmentText`], it is NOT touched by
/// [`repaint_segments`]: the sub-line keeps its dimmer style regardless of the active segment
/// (it is a quiet annotation, not the selection signal — the bold/fill active mark stays on the
/// LABEL).
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentSubText;

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
                    SegmentIndex(index)
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

/// Shows or hides a single segment of a [`SegmentedControl`] BY INDEX, MUTATING the
/// segment's [`Node::display`](bevy::ui::Node) in place — never despawning/respawning it
/// ([[ui-mutate-not-respawn]]).
///
/// A hidden segment is set to [`Display::None`], so the flex row/column COLLAPSES it: the
/// control visibly shows only the still-[`Display::Flex`] segments, with no gap left where
/// a hidden one was. This is the per-segment visibility a caller needs when a
/// [`SegmentedControl`]'s segments map to an OFFERED set that varies (e.g. a weapon's
/// fire modes) yet the segment entity ids must stay STABLE — the GTW-284 mutate-not-churn
/// invariant: the control is spawned once with ALL segments, and the offered subset is
/// revealed by toggling per-segment visibility rather than rebuilding the control.
///
/// It walks the control's `children` for the [`Segment`] whose [`SegmentIndex`] equals
/// `index` and sets its [`Display`]; an out-of-range index is a no-op. The write is
/// guarded so it marks the [`Node`] changed only on a REAL change (change-detection
/// hygiene). Returns whether a matching segment was found.
///
/// Caller-driven (the `set_*` helper idiom — no per-frame system; the bar / pips
/// precedent): build a
/// `SystemState<(Query<&Children>, Query<(&SegmentIndex, &mut Node), With<Segment>>)>`,
/// call this, then `state.apply(world)`. Param-only — no `&mut World` (bevy-traps rule 7).
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

/// Read-only [`Query`] data identifying one segment when setting its sub-line: its
/// [`SegmentIndex`] and its [`Children`] (to look for an existing [`SegmentSubText`] node).
///
/// Named to keep [`set_segment_sub_line`]'s signature legible (clippy `type_complexity`).
type SubLineSegment = (&'static SegmentIndex, &'static Children);

/// Sets, updates, or clears a single segment's OPTIONAL sub-line BY INDEX (GTW-303),
/// returning whether a matching segment was found.
///
/// The sub-line is a SECOND [`Text`](bevy::prelude::Text) node ([`SegmentSubText`]) STACKED
/// below the segment's primary [`SegmentText`] label, at a smaller font
/// ([`SEGMENT_SUB_FONT_PT`]) and a dimmer color ([`sub_line_color`] of the segment's base
/// text) so it reads as a quiet secondary line. It is created / mutated / removed IN PLACE —
/// never by despawning/respawning the SEGMENT ([[ui-mutate-not-respawn]]):
///
/// - `Some(label)` and the segment has NO sub-line yet → SPAWNS the sub-line node as a child
///   of the segment (so it stacks below the label in the segment's centered column).
/// - `Some(label)` and the segment ALREADY has a sub-line → MUTATES that existing node's
///   [`Text`] in place to the new caption (the sub-line entity id stays STABLE), only when
///   the text actually differs (change-detection hygiene).
/// - `None` → DESPAWNS the segment's sub-line node if present (clears the sub-line), leaving
///   the segment rendering as a single centered label again.
///
/// A segment with no sub-line has NO [`SegmentSubText`] node, so it renders exactly as it did
/// before GTW-303 — the stance control and any other pre-GTW-303 caller are unaffected. The
/// sub-line's color is derived from the control root's [`SegmentColors::base_text`] (dimmed),
/// so it matches the segment palette without the caller re-passing colors.
///
/// Caller-driven (the `set_*` helper idiom — the bar / pips / [`set_segment_visible`]
/// precedent): build a
/// `SystemState<(Commands, Query<(&Children, &SegmentColors)>, Query<SubLineSegment, With<Segment>>, Query<&mut Text, With<SegmentSubText>>)>`,
/// call this, then `state.apply(world)`. Param-only — no `&mut World` (bevy-traps rule 7); the
/// spawn / despawn go through [`Commands`].
#[allow(
    clippy::type_complexity,
    reason = "param tuple aliased where possible; the spawn/mutate/clear paths fix the \
    children + colors + sub-text query shapes"
)]
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
        // The segment's existing sub-line node, if any (a SegmentSubText child).
        let existing = seg_children.iter().find(|&kid| sub_texts.get(kid).is_ok());
        match (sub_line, existing) {
            // Set / update: mutate an existing sub-line in place, or spawn a fresh one.
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
                // GTW-322 — author the sub-line text child as a `bsn!` scene parented
                // under the segment (mirrors the converted segment label + the
                // `SwitchKnob` child). `Text::new` + the unit `SegmentSubText` marker +
                // `UiTextColor(dim)` ride the inline macro; the `TextFont` is NOT `Unpin`
                // (its `FontFeatures` / `FontVariations` fields), so it takes the
                // `template(move |_| Ok(value.clone()))` closure escape hatch. It is
                // queued as a `Children` related-scene on the existing `child` segment, so
                // the lazy "spawn once if absent" semantics are preserved.
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
            // Clear: despawn the sub-line node if present; nothing to do otherwise.
            (None, Some(node)) => {
                commands.entity(node).despawn();
            }
            (None, None) => {}
        }
        return true;
    }
    false
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
        font_size: FontSize::Px(SEGMENT_FONT_PT),
        weight: active_weight(is_active),
        ..default()
    }
}

/// The [`TextFont`](bevy::text::TextFont) for a segment's OPTIONAL sub-line (GTW-303): the
/// smaller [`SEGMENT_SUB_FONT_PT`] at normal weight — quieter than the bold-on-active label.
fn sub_line_font() -> TextFont {
    TextFont {
        font_size: FontSize::Px(SEGMENT_SUB_FONT_PT),
        weight: FontWeight::NORMAL,
        ..default()
    }
}

/// The DIMMED color of a segment's sub-line, derived from the segment's `base_text` color
/// (GTW-303): the same hue at [`SEGMENT_SUB_ALPHA`] opacity, so the sub-line reads as a
/// quieter second line that matches the palette without the caller re-passing a color.
fn sub_line_color(base_text: Color) -> Color {
    base_text.with_alpha(base_text.alpha() * SEGMENT_SUB_ALPHA)
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

/// The segment label font size, in typographic points.
const SEGMENT_FONT_PT: f32 = 16.0;

/// The segment SUB-LINE font size, in typographic points (GTW-303): ~11 pt — distinctly
/// smaller than the [`SEGMENT_FONT_PT`] label so the second line reads as a quiet annotation
/// (the TU cost under the firemode name). Font size in pt is the ONE permitted px exception to
/// the relative-units rule (`ui-responsive-not-px`), matching the label const.
const SEGMENT_SUB_FONT_PT: f32 = 11.0;

/// The opacity MULTIPLIER applied to a segment's `base_text` color to dim its sub-line
/// (GTW-303): 0.7 — visibly dimmer than the label without becoming unreadable.
const SEGMENT_SUB_ALPHA: f32 = 0.7;
