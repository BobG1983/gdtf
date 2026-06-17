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

mod builders;
mod markers;
mod paint;
#[cfg(test)]
mod test;

pub use builders::{spawn_button, spawn_panel};
pub use markers::{ActiveButton, ButtonLabel, DisabledButton};
pub use paint::{paint_active_buttons, paint_disabled_buttons};
