//! Types for segmented controls.

use bevy::prelude::*;

/// Primary label on a segment.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct SegmentLabel(String);

impl SegmentLabel {
    /// Wrap a label string.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

/// Optional secondary line under the primary label.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct SegmentSubLabel(String);

impl SegmentSubLabel {
    /// Wrap a sub-label string.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

/// Colors for active vs base segments.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct SegmentColors {
    /// Active segment background.
    pub active_bg: Color,
    /// Active segment text.
    pub active_text: Color,
    /// Inactive segment background.
    pub base_bg: Color,
    /// Inactive segment text.
    pub base_text: Color,
}

/// Marks the control root.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentedControl;

/// Index of the currently active segment.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActiveSegment(usize);

impl ActiveSegment {
    /// Wrap an index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Marks one segment button.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Segment;

/// Index of this segment within its control.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentIndex(usize);

impl SegmentIndex {
    /// Wrap an index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Marks primary segment text.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentText;

/// Marks secondary segment text.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SegmentSubText;

/// Emitted when a segment is selected.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SegmentSelected {
    /// Control entity.
    pub control: Entity,
    /// Selected segment index.
    pub index: SegmentIndex,
}
