//! The theme-derived button interaction layer.
//!
//! [`theme_interaction`] is the per-widget hover/press feedback system. It
//! composes **on top of** the base look painted by
//! [`apply_theme`](crate::themed::apply_theme): for every button whose
//! [`Interaction`](bevy::ui::Interaction) changed this frame it writes the
//! state-appropriate fill from the **button** sub-theme of the *current*
//! [`GdtfTheme`](crate::theme::GdtfTheme) —
//! [`None`](bevy::ui::Interaction::None) → the resting
//! [`ButtonColor`](crate::theme::ButtonColor),
//! [`Hovered`](bevy::ui::Interaction::Hovered) →
//! [`HoverColor`](crate::theme::HoverColor),
//! [`Pressed`](bevy::ui::Interaction::Pressed) →
//! [`PressedColor`](crate::theme::PressedColor).
//!
//! ## Live theme read (never a spawn snapshot)
//!
//! The fill is resolved from the resource on **every** run, so a hot-reload
//! that changes the palette is reflected the next time the system writes — a
//! hovered or pressed button never shows a stale color from the old theme.
//!
//! ## Ordering and guards
//!
//! It runs after the [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)
//! set (bevy-traps rule 3) so it lays its swap on the freshest base, and takes
//! the theme as `Option<Res<GdtfTheme>>` so it is inert before the resource is
//! populated (bevy-traps rule 1). [`DisabledButton`](crate::widgets::DisabledButton)
//! widgets are excluded with `Without<DisabledButton>`, so a disabled button
//! never hover/press-swaps.
//!
//! ## Mouse hover follows focus
//!
//! [`sync_hover_to_focus`] is the second system in this band (GTW-141): it moves
//! the [`InputFocus`](bevy::input_focus::InputFocus) resource onto whatever
//! enabled button the mouse is hovering, so pointer hover and keyboard / gamepad
//! navigation share one focus cursor. It composes with GTW-119's directional
//! navigation — both write `InputFocus`, neither fights the other, because each
//! is driven by a distinct change/event signal (this one by
//! `Changed<Interaction>`, that one by a [`NavigateRequest`](crate::focus_nav::NavigateRequest)).
//!
//! It does **not** write [`Interaction`](bevy::ui::Interaction) itself: that is
//! set by `bevy_ui`'s built-in `ui_focus_system` from raw mouse input in
//! `PreUpdate` (see bevy-traps), so by the time this `Update` system reads
//! `Changed<Interaction>` the cursor's hit for the frame is already resolved.

mod focus;
#[cfg(test)]
mod test;
mod theme;

pub use focus::sync_hover_to_focus;
pub use theme::theme_interaction;
