//! Segmented control (radio-style button group).

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
