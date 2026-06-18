//! The theme-derived button hover/press feedback system.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use crate::{
    theme::GdtfTheme,
    widgets::{ActiveButton, DisabledButton, Segment, Switch},
};

/// Query filter selecting the buttons [`theme_interaction`] restyles: enabled,
/// NOT-active buttons whose [`Interaction`](bevy::ui::Interaction) changed this frame.
///
/// Factored into a named alias both to keep the system signature legible
/// (clippy's `type_complexity`) and to make the exclusions explicit:
/// `Without<DisabledButton>` skips disabled buttons (AC#5), `Without<ActiveButton>`
/// skips toggled-on buttons so their color comes ONLY from
/// [`paint_active_buttons`](crate::widgets::paint_active_buttons) — the GTW-266
/// active-is-STICKY rule (no hover/press override flicker on an active toggle) —
/// `Without<Segment>` skips [`SegmentedControl`](crate::widgets::SegmentedControl)
/// segments so their fill comes ONLY from
/// [`repaint_segments`](crate::widgets::repaint_segments) — the GTW-277 segment-active
/// rule (a segment is a [`Button`](bevy::ui::widget::Button), so without this exclusion
/// the resting/hover fill would clobber the active segment's highlight) —
/// `Without<Switch>` skips [`Switch`](crate::widgets::Switch) tracks so their fill comes
/// ONLY from [`drive_switches`](crate::widgets::drive_switches) (and the caller's
/// sim-driven sync) — the GTW-277 switch-track rule (a switch track is itself a
/// [`Button`](bevy::ui::widget::Button), so without this exclusion the resting
/// `button.color` fill would clobber the switch's off/on track color the frame its
/// `Interaction` is added/changed, leaving the track near-invisible against the panel) —
/// and `Changed<Interaction>` limits the work to state transitions.
type InteractedButton = (
    Changed<Interaction>,
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
    Without<Segment>,
    Without<Switch>,
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
/// [`Interaction`](bevy::ui::Interaction) `Changed` this frame and is **none** of a
/// [`DisabledButton`](crate::widgets::DisabledButton), an
/// [`ActiveButton`](crate::widgets::ActiveButton), a
/// [`Segment`](crate::widgets::Segment) of a
/// [`SegmentedControl`](crate::widgets::SegmentedControl), **nor** a
/// [`Switch`](crate::widgets::Switch) track, and writes its
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
/// GTW-277 — segment fill is OWNED by the segmented control: the query EXCLUDES
/// [`Segment`](crate::widgets::Segment) (`Without<Segment>`), so a
/// [`SegmentedControl`](crate::widgets::SegmentedControl) segment — which IS a
/// [`Button`](bevy::ui::widget::Button) — never takes a resting/hover/press fill here;
/// its background comes solely from
/// [`repaint_segments`](crate::widgets::repaint_segments) (active = the filled active
/// background, else the base background). Without this exclusion the resting fill would
/// clobber the active segment's highlight the frame its `Interaction` was added (spawn)
/// or changed (hover), so the active segment never read as selected.
///
/// GTW-277 — switch track fill is OWNED by the switch driver: the query EXCLUDES
/// [`Switch`](crate::widgets::Switch) (`Without<Switch>`). A [`Switch`] track is itself
/// a [`Button`](bevy::ui::widget::Button) (so a click anywhere on the track flips it),
/// so without this exclusion the resting `button.color` fill would clobber the switch's
/// off/on track color the frame its `Interaction` was added (spawn) or changed (hover) —
/// leaving the track painted the near-panel `button.color` and reading as a bare knob
/// with no visible pill. Its background now comes solely from
/// [`drive_switches`](crate::widgets::drive_switches) (the click flip) and the caller's
/// sim-driven sync.
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
