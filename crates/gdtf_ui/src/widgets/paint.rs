//! The disabled / active fill paint passes that compose on top of the base look.

use bevy::{prelude::*, ui::BackgroundColor};

use super::markers::{ActiveButton, DisabledButton};
use crate::theme::GdtfTheme;

/// Paints every [`DisabledButton`] with the button sub-theme's explicit
/// [`DisabledColor`](crate::theme::DisabledColor) fill, writing it over the
/// button's [`BackgroundColor`](bevy::ui::BackgroundColor).
///
/// Runs **after** [`apply_theme`](crate::themed::apply_theme) (the
/// [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme) set) so it
/// composes on top of the freshest base fill (bevy-traps rule 3); a hot-reload
/// therefore re-paints the disabled fill from the new palette. The fill is the
/// live theme's data-driven [`DisabledColor`](crate::theme::DisabledColor) — a
/// muted color so a disabled control reads as inert.
///
/// Takes the theme as `Option<Res<GdtfTheme>>` so it is inert (rather than
/// panicking) before the resource is populated (bevy-traps rule 1).
pub fn paint_disabled_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut disabled: Query<&mut BackgroundColor, With<DisabledButton>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let fill = *theme.button.disabled;
    for mut background in &mut disabled {
        background.0 = fill;
    }
}

/// Paints every **active** ([`ActiveButton`]) but NOT-disabled button with the
/// button sub-theme's explicit [`ActiveColor`](crate::theme::ActiveColor) fill,
/// writing it over the button's [`BackgroundColor`](bevy::ui::BackgroundColor).
///
/// Runs **after** [`apply_theme`](crate::themed::apply_theme) (the
/// [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme) set) so it
/// composes on top of the freshest base fill (bevy-traps rule 3); a hot-reload
/// therefore re-paints the active fill from the new palette. The fill is the live
/// theme's data-driven [`ActiveColor`](crate::theme::ActiveColor), distinct from
/// the resting / hover / pressed / disabled fills.
///
/// **Disabled beats active:** the query filters `Without<DisabledButton>`, so a
/// button that is BOTH [`DisabledButton`] and [`ActiveButton`] is skipped here and
/// keeps the disabled fill that [`paint_disabled_buttons`] wrote. This system is
/// purely a *paint* — it touches no [`Interaction`](bevy::ui::Interaction), so an active
/// button stays fully interactive (clickable to toggle off).
///
/// **Active is STICKY (GTW-266):** the interaction-feedback system
/// [`theme_interaction`](crate::interaction::theme_interaction) EXCLUDES `ActiveButton`
/// (`Without<ActiveButton>`) and this paint is ordered `.after(theme_interaction)`, so an
/// active button's `BackgroundColor` comes ONLY from here — a hover/press swap never
/// clobbers the active fill (no flicker).
///
/// Takes the theme as `Option<Res<GdtfTheme>>` so it is inert (rather than
/// panicking) before the resource is populated (bevy-traps rule 1).
pub fn paint_active_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut active: Query<&mut BackgroundColor, (With<ActiveButton>, Without<DisabledButton>)>,
) {
    let Some(theme) = theme else {
        return;
    };

    let fill = *theme.button.active;
    for mut background in &mut active {
        background.0 = fill;
    }
}
