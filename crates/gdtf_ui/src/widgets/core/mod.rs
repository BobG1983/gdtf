//! Core widget types and spawn helpers.

mod builders;
mod markers;
mod orientation;
mod paint;
mod pips;
mod progress_bar;
mod segmented_control;
mod switch;
#[cfg(test)]
pub(crate) mod test_support;

pub use builders::{spawn_button, spawn_panel};
pub use markers::{ActiveButton, ButtonLabel, DisabledButton};
pub use orientation::Orientation;
pub use paint::{paint_active_buttons, paint_disabled_buttons};
pub use pips::{FilledPips, Pip, PipsRow, set_pips, spawn_pips};
pub use progress_bar::{
    FillFraction, ProgressBarFill, ProgressBarTrack, set_progress_bar, spawn_progress_bar,
};
pub use segmented_control::{
    ActiveSegment, Segment, SegmentColors, SegmentIndex, SegmentLabel, SegmentSelected,
    SegmentSubLabel, SegmentSubText, SegmentText, SegmentedControl, repaint_segments,
    select_segment_on_press, set_segment_sub_line, set_segment_visible, spawn_segmented_control,
};
pub use switch::{
    Switch, SwitchColors, SwitchKnob, SwitchOrientation, SwitchState, ToggleFlipped,
    drive_switches, spawn_switch,
};
