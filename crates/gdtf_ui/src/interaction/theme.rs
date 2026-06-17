//! The theme-derived button hover/press feedback system.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use crate::{
    theme::GdtfTheme,
    widgets::{ActiveButton, DisabledButton},
};

/// Query filter selecting the buttons [`theme_interaction`] restyles: enabled,
/// NOT-active buttons whose [`Interaction`](bevy::ui::Interaction) changed this frame.
///
/// Factored into a named alias both to keep the system signature legible
/// (clippy's `type_complexity`) and to make the exclusions explicit:
/// `Without<DisabledButton>` skips disabled buttons (AC#5), `Without<ActiveButton>`
/// skips toggled-on buttons so their color comes ONLY from
/// [`paint_active_buttons`](crate::widgets::paint_active_buttons) — the GTW-266
/// active-is-STICKY rule (no hover/press override flicker on an active toggle) — and
/// `Changed<Interaction>` limits the work to state transitions.
type InteractedButton = (
    Changed<Interaction>,
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
);

/// The per-button visuals [`theme_interaction`] reads and writes: the current
/// [`Interaction`](bevy::ui::Interaction) plus the
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and
/// [`BorderColor`](bevy::ui::BorderColor) it layers the theme state-fill onto.
type InteractionVisuals = (
    &'static Interaction,
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
);

/// The SINGLE source of truth for the button-state → fill mapping (GTW-280, AC2).
///
/// Maps a button's current [`Interaction`](bevy::ui::Interaction) to the matching
/// fill [`Color`](bevy::prelude::Color) from the button sub-theme of the live
/// [`GdtfTheme`](crate::theme::GdtfTheme):
///
/// - [`Interaction::None`](bevy::ui::Interaction::None) → the resting
///   [`ButtonColor`](crate::theme::ButtonColor).
/// - [`Interaction::Hovered`](bevy::ui::Interaction::Hovered) →
///   [`HoverColor`](crate::theme::HoverColor).
/// - [`Interaction::Pressed`](bevy::ui::Interaction::Pressed) →
///   [`PressedColor`](crate::theme::PressedColor).
///
/// Both [`theme_interaction`] (the hover/press feedback) and
/// [`repaint_deactivated_buttons`](crate::interaction::repaint_deactivated_buttons)
/// (the GTW-280 deactivation repaint) resolve a button's fill through this one
/// helper, so the two systems can never drift apart — there is no duplicated
/// `match` to keep in sync.
pub(crate) fn interaction_fill(theme: &GdtfTheme, interaction: Interaction) -> Color {
    match interaction {
        Interaction::None => *theme.button.color,
        Interaction::Hovered => *theme.button.hover,
        Interaction::Pressed => *theme.button.pressed,
    }
}

/// Lays theme-derived hover/press feedback on top of the base button look.
///
/// Queries every [`Button`](bevy::ui::Button) whose
/// [`Interaction`](bevy::ui::Interaction) `Changed` this frame and is **neither** a
/// [`DisabledButton`](crate::widgets::DisabledButton) **nor** an
/// [`ActiveButton`](crate::widgets::ActiveButton), and writes its
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
/// GTW-266 — active is STICKY: the query EXCLUDES
/// [`ActiveButton`](crate::widgets::ActiveButton) (`Without<ActiveButton>`), so a
/// toggled-on button NEVER takes a hover/press swap here; its color comes solely from
/// [`paint_active_buttons`](crate::widgets::paint_active_buttons). That removes the
/// one-frame flicker the unordered active-vs-interaction writers produced when a button
/// was BOTH active and hovered, and gives the deterministic toggle-button UX the
/// Mode/Stance toggle panels need.
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] ordered
/// `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)` and
/// `.before(`[`paint_active_buttons`](crate::widgets::paint_active_buttons)`)` (the active
/// paint is the LAST writer), guarded by `Option<Res<GdtfTheme>>` for the absent-resource
/// case (bevy-traps rules 1 and 3).
pub fn theme_interaction(
    theme: Option<Res<GdtfTheme>>,
    mut buttons: Query<InteractionVisuals, InteractedButton>,
) {
    let Some(theme) = theme else {
        return;
    };

    for (interaction, mut background, mut border) in &mut buttons {
        background.0 = interaction_fill(&theme, *interaction);
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}
