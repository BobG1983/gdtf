pub mod focus_nav;
pub mod menu_nav;
pub mod theming;
pub mod widgets;

mod plugin;

pub use menu_nav::{MenuItem, MenuName, MenuScreen};
pub use plugin::UiPlugin;
pub use theming::{
    retheme::{resolve_theme_spec, theme_hot_ron_chain},
    theme, themed,
    themed::any_themed_added,
};
pub use widgets::{
    core::{
        ActiveButton, ActiveSegment, ButtonLabel, DisabledButton, FillFraction, FilledPips,
        Orientation, Pip, PipsRow, ProgressBarFill, ProgressBarTrack, Segment, SegmentColors,
        SegmentIndex, SegmentLabel, SegmentSelected, SegmentSubLabel, SegmentSubText, SegmentText,
        SegmentedControl, Switch, SwitchColors, SwitchKnob, SwitchOrientation, SwitchState,
        ToggleFlipped, drive_switches, paint_active_buttons, paint_disabled_buttons,
        repaint_segments, select_segment_on_press, set_pips, set_progress_bar,
        set_segment_sub_line, set_segment_visible, spawn_button, spawn_panel, spawn_pips,
        spawn_progress_bar, spawn_segmented_control, spawn_switch,
    },
    interaction::{repaint_deactivated_buttons, sync_hover_to_focus, theme_interaction},
};
