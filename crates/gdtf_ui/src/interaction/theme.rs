//! The theme-derived button hover/press feedback system.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use crate::{theme::GdtfTheme, widgets::DisabledButton};

/// Query filter selecting the buttons [`theme_interaction`] restyles: enabled
/// buttons whose [`Interaction`](bevy::ui::Interaction) changed this frame.
///
/// Factored into a named alias both to keep the system signature legible
/// (clippy's `type_complexity`) and to make the exclusion explicit:
/// `Without<DisabledButton>` is what skips disabled buttons (AC#5), and
/// `Changed<Interaction>` is what limits the work to state transitions.
type InteractedButton = (Changed<Interaction>, With<Button>, Without<DisabledButton>);

/// The per-button visuals [`theme_interaction`] reads and writes: the current
/// [`Interaction`](bevy::ui::Interaction) plus the
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and
/// [`BorderColor`](bevy::ui::BorderColor) it layers the theme state-fill onto.
type InteractionVisuals = (
    &'static Interaction,
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
);

/// Lays theme-derived hover/press feedback on top of the base button look.
///
/// Queries every [`Button`](bevy::ui::Button) whose
/// [`Interaction`](bevy::ui::Interaction) `Changed` this frame and is **not** a
/// [`DisabledButton`](crate::widgets::DisabledButton), and writes its
/// [`BackgroundColor`](bevy::ui::BackgroundColor) (and re-affirms its
/// [`BorderColor`](bevy::ui::BorderColor)) from the **current**
/// [`GdtfTheme`](crate::theme::GdtfTheme):
///
/// - [`Interaction::None`](bevy::ui::Interaction::None) → the resting
///   [`ButtonColor`](crate::theme::ButtonColor) base.
/// - [`Interaction::Hovered`](bevy::ui::Interaction::Hovered) →
///   [`HoverColor`](crate::theme::HoverColor).
/// - [`Interaction::Pressed`](bevy::ui::Interaction::Pressed) →
///   [`PressedColor`](crate::theme::PressedColor).
///
/// All three fills are sourced from the button sub-theme of the RON-driven theme
/// — there are no hardcoded hover/press literals. The theme is read live each run
/// (never snapshotted at spawn), so re-running after a palette change re-derives
/// the fill; the border is re-affirmed from the button sub-theme's
/// [`BorderColor`](crate::theme::BorderColor).
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] ordered
/// `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)`
/// and guarded by `Option<Res<GdtfTheme>>` for the absent-resource case
/// (bevy-traps rules 1 and 3).
pub fn theme_interaction(
    theme: Option<Res<GdtfTheme>>,
    mut buttons: Query<InteractionVisuals, InteractedButton>,
) {
    let Some(theme) = theme else {
        return;
    };

    for (interaction, mut background, mut border) in &mut buttons {
        let fill = match interaction {
            Interaction::None => *theme.button.color,
            Interaction::Hovered => *theme.button.hover,
            Interaction::Pressed => *theme.button.pressed,
        };
        background.0 = fill;
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}
