//! Hand-rolled `bevy_ui` layer for GDTF.
//!
//! This crate is the seam the menu / HUD work hangs on. It owns no combat rules
//! (those live in `gdtf_battle_sim`) and deliberately depends on **bevy only**,
//! so `gdtf_app` can depend on it without forming a dependency cycle.
//!
//! [`UiPlugin`] is the single registration seam: today it installs the
//! [`focus_nav`] sub-plugin ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)) and
//! nothing else. Later tickets attach further UI systems, resources, and assets
//! to [`UiPlugin`].
//!
//! The [`focus_nav`] module wires Bevy's `input_focus` framework and bridges
//! keyboard + gamepad input onto directional focus navigation; see its docs for
//! the activation-message decision.
//!
//! The data-driven [`theme`] module defines the on-disk theme schema, the runtime
//! [`GdtfTheme`](theme::GdtfTheme) resource, and the pure spec-to-resource
//! resolution; population of that resource lands with later tickets.
//!
//! The [`themed`] module owns the [`Themed`](themed::Themed) marker and the
//! central [`apply_theme`](themed::apply_theme) system — the hot-reload seam that
//! paints theme-derived visuals onto themed entities from the live
//! [`GdtfTheme`](theme::GdtfTheme).
//!
//! The [`theming::retheme`] module owns the live-reapply logic
//! ([`redrive_theme_on_asset_event`](theming::retheme::redrive_theme_on_asset_event)): on
//! an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for the active theme
//! asset it re-derives [`GdtfTheme`](theme::GdtfTheme) in place, and the
//! change-driven [`apply_theme`](themed::apply_theme) repaints every
//! [`Themed`](themed::Themed) entity the same frame — no restart (GTW-137).
//!
//! The [`widgets::core`] module owns the reusable spawn helpers
//! ([`spawn_panel`](widgets::core::spawn_panel) / [`spawn_button`](widgets::core::spawn_button)),
//! the [`DisabledButton`](widgets::core::DisabledButton) / [`ActiveButton`](widgets::core::ActiveButton)
//! markers, and their paint passes
//! ([`paint_disabled_buttons`](widgets::core::paint_disabled_buttons) /
//! [`paint_active_buttons`](widgets::core::paint_active_buttons)); the [`widgets::interaction`]
//! module owns the theme-derived hover/press feedback system. All compose *on top
//! of* [`apply_theme`](themed::apply_theme)'s base look, ordered after it. It also
//! owns the GTW-276 generic HUD widgets — [`ProgressBar`](widgets::core::spawn_progress_bar),
//! [`Pips`](widgets::core::spawn_pips), [`Switch`](widgets::core::Switch), and
//! [`SegmentedControl`](widgets::core::SegmentedControl) — the color-parameterized,
//! mutate-in-place building blocks the status / hover panels and the action bar reuse, plus
//! the GTW-410 [`Dropdown<T>`](widgets::core::Dropdown) combobox (a floating, above-panels
//! option list generic over the option identity). The dropdown's per-option-id drivers are
//! wired by [`register_dropdown::<T>`](register_dropdown); its type-agnostic systems ride
//! [`UiPlugin`].
//!
//! The [`UiPlugin`] registration seam itself lives in the private `plugin`
//! submodule and is re-exported here unchanged.

pub mod focus_nav;
pub mod theming;
pub mod widgets;

mod plugin;

// Module re-exports — preserve `gdtf_ui::theme::*` and `gdtf_ui::themed::*`
// sub-paths for the 27+ external callers that reach `GdtfTheme`, `default_theme`,
// `UiSystems`, etc. via the old root-level module path (GTW-385).
pub use plugin::{UiPlugin, register_dropdown};
pub use theming::{retheme::redrive_theme_on_asset_event, theme, themed, themed::any_themed_added};
pub use widgets::{
    core::{
        Accordion, AccordionAnim, AccordionColors, AccordionContent, AccordionHeader,
        AccordionProgress, AccordionRow, AccordionTarget, ActiveButton, ActiveSegment, ButtonLabel,
        Caret, CommittedNumericValue, CommittedTextValue, DisabledButton, Dropdown, DropdownAnchor,
        DropdownBackdrop, DropdownColors, DropdownDismissRequest, DropdownItem, DropdownLabel,
        DropdownOption, DropdownOptionLabel, DropdownOptions, DropdownPopup,
        DropdownSelectionChanged, DropdownState, EditBuffer, FieldColors, FieldText, FillFraction,
        FilledPips, NumericField, NumericFieldCommitted, NumericRange, NumericValue, OptionId,
        Orientation, Pip, PipsRow, ProgressBarFill, ProgressBarTrack, ScrollList, ScrollListArea,
        ScrollListBar, ScrollListColors, Segment, SegmentColors, SegmentIndex, SegmentLabel,
        SegmentSelected, SegmentSubLabel, SegmentSubText, SegmentText, SegmentedControl,
        SelectedIndex, Switch, SwitchColors, SwitchKnob, SwitchOrientation, SwitchState, TextField,
        TextFieldCommitted, ToggleFlipped, activate_focused_option, any_dropdown_open,
        close_dropdowns_on_dismiss_request, commit_on_focus_lost, dismiss_dropdowns_on_escape,
        dismiss_on_backdrop_press, drive_accordions, drive_switches, focus_field_on_press,
        handle_text_field_key, open_dropdown, paint_active_buttons, paint_disabled_buttons,
        position_dropdown_popups, register_numeric_field, register_text_field, repaint_segments,
        select_option_on_press, select_segment_on_press, set_pips, set_progress_bar,
        set_segment_sub_line, set_segment_visible, spawn_accordion, spawn_accordion_row,
        spawn_button, spawn_dropdown, spawn_numeric_field, spawn_panel, spawn_pips,
        spawn_progress_bar, spawn_scroll_list, spawn_segmented_control, spawn_switch,
        spawn_text_field, sync_edit_buffer_to_text,
    },
    interaction::{repaint_deactivated_buttons, sync_hover_to_focus, theme_interaction},
};
