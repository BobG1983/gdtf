//! The [`SegmentedControl`] widget vocabulary: label newtypes, colors, markers,
//! indices, and the [`SegmentSelected`] selection message.

use bevy::prelude::*;

/// The caption of one [`SegmentedControl`] segment.
///
/// A named newtype over the label string (mirroring
/// [`ButtonLabel`](crate::widgets::core::ButtonLabel), no-bare-types rule): a segment's label is a
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
/// cleared in place via [`set_segment_sub_line`](super::set_segment_sub_line).
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
/// Pure UI plumbing ([`Color`](bevy::prelude::Color)s): the active segment gets `active_bg` +
/// `active_text` (and **bold** weight), every other segment gets `base_bg` +
/// `base_text` (normal weight). The bold weight on the active segment is the
/// color-blind-safe second channel the contract requires (NOT color alone). Stored as
/// a [`Component`] on the control root so [`repaint_segments`](super::repaint_segments)
/// re-derives every segment's look from the active index without the caller re-passing colors.
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
/// The root lays its segment children out along
/// [`Orientation::flex_direction`](crate::widgets::core::Orientation::flex_direction); each
/// child is a [`Segment`]. The caller attaches its own identity marker alongside this
/// so the [`SegmentSelected`] listener can map the control to an action.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentedControl;

/// The currently-active segment index, held on the [`SegmentedControl`] ROOT.
///
/// [`repaint_segments`](super::repaint_segments) reacts to a CHANGE of this component
/// (`Changed<ActiveSegment>`)
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
    /// [`repaint_segments`](super::repaint_segments) repaints the same frame. The index is
    /// clamped by [`repaint_segments`](super::repaint_segments)' equality check (an
    /// out-of-range value simply highlights nothing), so no clamp is needed here.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Marker on one segment (a [`Button`](bevy::ui::widget::Button) child of a
/// [`SegmentedControl`] root).
///
/// Each segment carries its [`SegmentIndex`]; [`repaint_segments`](super::repaint_segments)
/// re-styles it by comparing its index to the root's [`ActiveSegment`].
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

impl SegmentIndex {
    /// Wraps a segment index — the producer-side constructor (symmetric with
    /// [`ActiveSegment::new`] and the field commits' `new`), so a caller that drives a selection
    /// (or a test exercising a [`SegmentSelected`] listener's real code path) can synthesize the
    /// reported index without reaching the private inner.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Marker on the LABEL text child of a [`Segment`].
///
/// [`repaint_segments`](super::repaint_segments) toggles its
/// [`TextFont`](bevy::text::TextFont) weight
/// (bold on the active segment, normal otherwise — the color-blind-safe channel) and
/// its [`TextColor`](bevy::text::TextColor).
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentText;

/// Marker on the OPTIONAL sub-line text child of a [`Segment`] (GTW-303).
///
/// A second [`Text`](bevy::prelude::Text) node STACKED below the segment's primary
/// [`SegmentText`] label (the segment [`Button`](bevy::ui::widget::Button) is a centered
/// [`Column`](bevy::ui::FlexDirection::Column)),
/// rendered at a SMALLER font (`SEGMENT_SUB_FONT_PT`) and a
/// DIMMER color (`sub_line_color`)
/// than the label so it reads as a quiet secondary line. It is created / removed in place by
/// [`set_segment_sub_line`](super::set_segment_sub_line) — a segment with no sub-line has NO
/// node carrying this marker, so it
/// renders exactly as a single centered label (the stance control + any pre-GTW-303 caller are
/// visually unchanged). UNLIKE [`SegmentText`], it is NOT touched by
/// [`repaint_segments`](super::repaint_segments): the sub-line keeps its dimmer style regardless
/// of the active segment
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
