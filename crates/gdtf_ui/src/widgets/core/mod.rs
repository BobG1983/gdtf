//! Reusable, themed widget builders, the button markers, and their paint
//! passes.
//!
//! This module owns the two spawn helpers the menu / HUD work builds its UI
//! from — [`spawn_panel`] and [`spawn_button`] — plus the [`DisabledButton`] /
//! [`ActiveButton`] markers and the [`paint_disabled_buttons`] /
//! [`paint_active_buttons`] systems that paint them.
//!
//! ## The Themed paint pass (no captured colors)
//!
//! The builders attach the [`Themed`](crate::themed::Themed) marker (GTW-135) and
//! write only *initial* theme-derived colors; the central
//! [`apply_theme`](crate::themed::apply_theme) system re-derives and re-writes them
//! from the live [`GdtfTheme`](crate::theme::GdtfTheme) on every run, so a
//! hot-reload re-paints every widget. Post-GTW-149 a panel is painted from the
//! **panel** sub-theme and a button from the **button** sub-theme — distinct
//! boxes, not one shared "panel" look.
//!
//! ## Disabled buttons
//!
//! A [`DisabledButton`] is painted by [`paint_disabled_buttons`] with the button
//! sub-theme's explicit [`DisabledColor`](crate::theme::DisabledColor) fill
//! (re-applied after [`apply_theme`](crate::themed::apply_theme)) and is skipped
//! entirely by the interaction layer (which filters `Without<DisabledButton>`). It
//! stays [`Themed`](crate::themed::Themed), so the base-look pass still reaches it.
//!
//! ## Active (toggled-on) buttons
//!
//! An [`ActiveButton`] is painted by [`paint_active_buttons`] with the button
//! sub-theme's explicit [`ActiveColor`](crate::theme::ActiveColor) fill (re-applied
//! after [`apply_theme`](crate::themed::apply_theme)), so a button whose toggle is
//! ON reads as persistently engaged. UNLIKE [`DisabledButton`] it is **purely
//! visual** — it is NOT in any `Without<…>` interaction filter, so an active button
//! is still clickable (you click it to toggle OFF). When a button is BOTH
//! [`DisabledButton`] and [`ActiveButton`], DISABLED wins:
//! [`paint_active_buttons`] filters `Without<DisabledButton>`, so a disabled+active
//! button keeps the disabled fill.
//!
//! ## Generic HUD widgets (GTW-276)
//!
//! Four generic, color-parameterized HUD widgets the status panel, hover panel, and
//! action bar build on. Each is pure VIEW and updates by MUTATING its existing
//! entities — never despawn/respawn on a value change:
//!
//! - [`ProgressBar`](spawn_progress_bar): a track + fill; [`set_progress_bar`]
//!   mutates the fill width to a [`FillFraction`] (the TU / HP bars).
//! - [`Pips`](spawn_pips): a row of N circle nodes; [`set_pips`] re-colors them by an
//!   M-of-N split ([`FilledPips`]) (the Wounds pips).
//! - [`Switch`](spawn_switch): a knob in a rounded track; clicking flips it and emits
//!   a [`ToggleFlipped`] message carrying the switch's identity (the Aim toggle).
//! - [`SegmentedControl`](spawn_segmented_control): N labeled segments, one active;
//!   selecting one repaints ALL segments the same frame and emits a
//!   [`SegmentSelected`] message (the Fire-Mode / stance controls).
//!
//! [`Switch`] and [`SegmentedControl`] each have a driver system registered by
//! [`UiPlugin`](crate::UiPlugin); the bar and pips have no per-frame system (they are
//! updated by the caller through their `set_*` helpers).
//!
//! (The GTW-410 `Dropdown<T>` combobox, the GTW-411 `TextField`/`NumericField` editable
//! fields, the GTW-412 `ScrollList` container, and the GTW-416 `Accordion` were RETIRED by
//! GTW-655/GTW-636: their only consumers — the GTW-434 procgen visualizer and the in-game
//! gang editor — moved off / were retired, and the widget census found no other user.)

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
