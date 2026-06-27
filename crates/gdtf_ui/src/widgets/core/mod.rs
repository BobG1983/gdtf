//! Reusable, theme-seam widget builders, the button markers, and their paint
//! passes.
//!
//! This module owns the two spawn helpers the menu / HUD work builds its UI
//! from — [`spawn_panel`] and [`spawn_button`] — plus the [`DisabledButton`] /
//! [`ActiveButton`] markers and the [`paint_disabled_buttons`] /
//! [`paint_active_buttons`] systems that paint them.
//!
//! ## The Themed seam (no captured colors)
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
//! ## Dropdown / combobox (GTW-410)
//!
//! - [`Dropdown<T>`](dropdown::Dropdown): a themed closed control showing the current
//!   selection; clicking opens a FLOATING option list ABOVE sibling panels (via
//!   [`GlobalZIndex`](bevy::ui::GlobalZIndex) strictly above the contextual panel — the
//!   bevy-traps #8 occlusion guard). Selecting an option closes the list, mutates the shown
//!   label in place, and emits a typed [`DropdownSelectionChanged<T>`](dropdown::DropdownSelectionChanged);
//!   an outside click (a full-screen backdrop) dismisses without changing the selection.
//!   Generic over the option IDENTITY; keyboard focus / arrow-nav / Enter / Esc all reuse the
//!   existing [`focus_nav`](crate::focus_nav) helpers. Its open / select / dismiss / position
//!   drivers are registered by [`UiPlugin`](crate::UiPlugin) per option-id type.
//!
//! ## Editable text / numeric fields (GTW-411)
//!
//! - [`TextField`](text_field::TextField) ([`spawn_text_field`]): a themed, focus-driven
//!   editable text box assembled from the [`bevy_input_focus`](bevy::input_focus) primitives
//!   (Bevy 0.19 has no built-in text input). Pressing it captures focus; typed characters
//!   APPEND to its [`EditBuffer`](text_field::EditBuffer), Backspace pops, a rendered
//!   [`Caret`](text_field::Caret) sits at the text END. Enter / blur commit a typed
//!   [`CommittedTextValue`](text_field::CommittedTextValue) in a
//!   [`TextFieldCommitted`](text_field::TextFieldCommitted) message; Escape reverts.
//! - [`NumericField`](text_field::NumericField) ([`spawn_numeric_field`]): the same editing
//!   model, but it CLAMPS the parsed buffer to a [`NumericRange`](text_field::NumericRange) and
//!   commits a typed [`CommittedNumericValue`](text_field::CommittedNumericValue) in a
//!   [`NumericFieldCommitted`](text_field::NumericFieldCommitted) message; invalid / empty
//!   input reverts to the last-good value (never panics).
//!
//! The type-agnostic pieces ride [`register_text_field`]; each numeric `N` registers via
//! [`register_numeric_field::<N>`].
//!
//! ## Scrollable list container (GTW-412)
//!
//! - [`ScrollList`](scroll_list::ScrollList) ([`spawn_scroll_list`]): a themed frame that
//!   CLIPS + SCROLLS a vertical stack of caller-supplied rows taller than it, with a
//!   draggable / wheel-driven scrollbar. Rows are OPAQUE to the list — the caller parents
//!   its own row bundles onto the returned [`ScrollListArea`](scroll_list::ScrollListArea)
//!   entity and the list only stacks + clips them. Built on Bevy's built-in
//!   [`ScrollArea`](bevy::ui_widgets::ScrollArea) /
//!   [`Scrollbar`](bevy::ui_widgets::Scrollbar) (the scroll mechanism is NOT hand-rolled);
//!   their plugins ride [`UiPlugin`](crate::UiPlugin). All sizing is relative bar the two
//!   API-forced scrollbar-chrome px the engine defines as pixels. The expand/collapse lerp
//!   accordion is explicitly OUT of scope (GTW-403 child / GTW-416).

mod builders;
mod dropdown;
mod markers;
mod orientation;
mod paint;
mod pips;
mod progress_bar;
mod scroll_list;
mod segmented_control;
mod switch;
#[cfg(test)]
mod test;
#[cfg(test)]
mod test_hud;
mod text_field;

pub use builders::{spawn_button, spawn_panel};
pub use dropdown::{
    Dropdown, DropdownAnchor, DropdownBackdrop, DropdownColors, DropdownDismissRequest,
    DropdownItem, DropdownLabel, DropdownOption, DropdownOptionLabel, DropdownOptions,
    DropdownPopup, DropdownSelectionChanged, DropdownState, OptionId, SelectedIndex,
    activate_focused_option, any_dropdown_open, close_dropdowns_on_dismiss_request,
    dismiss_dropdowns_on_escape, dismiss_on_backdrop_press, open_dropdown,
    position_dropdown_popups, select_option_on_press, spawn_dropdown,
};
pub use markers::{ActiveButton, ButtonLabel, DisabledButton};
pub use orientation::Orientation;
pub use paint::{paint_active_buttons, paint_disabled_buttons};
pub use pips::{FilledPips, Pip, PipsRow, set_pips, spawn_pips};
pub use progress_bar::{
    FillFraction, ProgressBarFill, ProgressBarTrack, set_progress_bar, spawn_progress_bar,
};
pub use scroll_list::{
    ScrollList, ScrollListArea, ScrollListBar, ScrollListColors, spawn_scroll_list,
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
pub use text_field::{
    Caret, CommittedNumericValue, CommittedTextValue, EditBuffer, FieldColors, FieldText,
    NumericField, NumericFieldCommitted, NumericRange, NumericValue, TextField, TextFieldCommitted,
    commit_on_focus_lost, focus_field_on_press, handle_text_field_key, register_numeric_field,
    register_text_field, spawn_numeric_field, spawn_text_field, sync_edit_buffer_to_text,
};
