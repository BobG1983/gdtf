//! The [`SegmentedControl`] widget: N adjacent labeled segments, exactly one active.
//!
//! A segmented control is the Fire-Mode (Single/Burst/Full-Auto) and the stance
//! (Stand/Kneel/Prone) controls in the mockup: adjacent LABELED segments where
//! exactly one is active. The active segment reads as a FILLED background **plus
//! bold** text — NOT color alone, so it is color-blind-safe. The label sits INSIDE
//! each segment, so the label IS the hit target. Orientation is a flex-direction
//! param ([`Orientation::Horizontal`](super::Orientation::Horizontal) = Row,
//! [`Orientation::Vertical`](super::Orientation::Vertical) = Column).
//!
//! Changing the active segment repaints ALL segments the SAME frame
//! ([`repaint_segments`] reacts to the [`ActiveSegment`] index change on the root,
//! NOT to hover — the GTW-280/284 lesson: the de-selected segment returns to its base
//! look immediately). All repaints MUTATE existing segment entities in place; nothing
//! is despawned/respawned on a selection change ([[ui-mutate-not-respawn]]).
//!
//! Selecting a new segment emits a [`SegmentSelected`] message carrying the chosen
//! segment's IDENTITY (the control [`Entity`](bevy::prelude::Entity) + the segment
//! index), so a downstream listener maps it to an action; `gdtf_ui` defines the
//! message, never the act.

mod interaction;
mod mutators;
mod spawn;
mod style;
#[cfg(test)]
mod test;
mod types;

pub use interaction::{repaint_segments, select_segment_on_press};
pub use mutators::{set_segment_sub_line, set_segment_visible};
pub use spawn::spawn_segmented_control;
pub use types::{
    ActiveSegment, Segment, SegmentColors, SegmentIndex, SegmentLabel, SegmentSelected,
    SegmentSubLabel, SegmentSubText, SegmentText, SegmentedControl,
};
