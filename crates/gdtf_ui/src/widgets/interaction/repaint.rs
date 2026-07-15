//! The deactivation-repaint system (GTW-280): repaint a just-de-selected toggle
//! the SAME frame it loses [`ActiveButton`], without waiting for a hover; and the
//! theme-reload repaint (GTW-147): repaint every enabled button to its
//! interaction-correct fill the SAME frame [`GdtfTheme`] changes, so a button held
//! `Hovered`/`Pressed` across a hot-reload does not get stuck on the new theme's
//! resting base until the user re-hovers.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use super::theme::interaction_fill;
use crate::{
    theme::GdtfTheme,
    widgets::core::{ActiveButton, DisabledButton, Segment, Switch},
};

/// Query filter selecting the buttons [`repaint_deactivated_buttons`] repaints:
/// enabled, NOT-active buttons (so they are no longer painted by
/// [`paint_active_buttons`](crate::paint_active_buttons)).
///
/// Factored into a named alias both to keep the system signature legible
/// (clippy's `type_complexity`) and to make the exclusions explicit:
/// `With<Button>` restricts to real buttons, `Without<DisabledButton>` leaves
/// disabled buttons to [`paint_disabled_buttons`](crate::paint_disabled_buttons),
/// `Without<ActiveButton>` confirms the button is no longer active (the
/// `RemovedComponents` signal fires the frame the marker is removed, by which
/// time the component is already gone), and `Without<Segment>` leaves
/// [`SegmentedControl`](crate::SegmentedControl) segment fills to
/// [`repaint_segments`](crate::repaint_segments) (GTW-277 — a segment IS a
/// [`Button`](bevy::ui::widget::Button), so without this it would steal a segment's
/// active highlight). A despawned entity also surfaces in
/// [`RemovedComponents`](bevy::prelude::RemovedComponents) but misses this query,
/// so it is silently skipped.
type DeactivatedButton = (
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
    Without<Segment>,
);

/// The per-button visuals [`repaint_deactivated_buttons`] reads and writes: the
/// current [`Interaction`](bevy::ui::Interaction) plus the
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and
/// [`BorderColor`](bevy::ui::BorderColor) it re-derives the resting/hover/pressed
/// fill onto.
type DeactivationVisuals = (
    &'static Interaction,
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
);

/// Repaints a button the frame it LOSES [`ActiveButton`], from its CURRENT
/// [`Interaction`](bevy::ui::Interaction) (GTW-280).
///
/// Reads [`RemovedComponents`](bevy::prelude::RemovedComponents)`<`[`ActiveButton`](crate::ActiveButton)`>`
/// and, for each just-deactivated entity that is STILL present and matches
/// `(With<Button>, Without<DisabledButton>, Without<ActiveButton>)`, writes its
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and re-affirms its
/// [`BorderColor`](bevy::ui::BorderColor) from its current
/// [`Interaction`](bevy::ui::Interaction) using the SAME button-state → fill
/// mapping as [`theme_interaction`](crate::theme_interaction) — the
/// shared [`interaction_fill`] helper, so the mapping is never duplicated (AC2).
///
/// ## Why this system exists
///
/// [`paint_active_buttons`](crate::paint_active_buttons) writes only
/// buttons `With<ActiveButton>`, and
/// [`theme_interaction`](crate::theme_interaction) runs only on
/// `Changed<Interaction>`. So the deactivation transition — a sibling toggle
/// becomes active while THIS button's `Interaction` is unchanged — was handled by
/// NOTHING: the just-de-selected toggle kept its stale active fill until a hover
/// changed its `Interaction` and `theme_interaction` fired. The user saw both the
/// old and new toggle reading as selected on the Mode and Stance panels. This
/// system closes that gap generically in `gdtf_ui`, so Mode, Stance, Aim, and the
/// future `SegmentedControl` all inherit correct deactivation repaint.
///
/// ## Skips and disjointness
///
/// A DESPAWNED entity also fires `RemovedComponents`; it is skipped by the query
/// miss (`get_mut` returns `Err`). A DISABLED button is left to
/// [`paint_disabled_buttons`](crate::paint_disabled_buttons)
/// (`Without<DisabledButton>`). A button that is somehow STILL active (e.g.
/// removed-then-re-added the same frame) is skipped (`Without<ActiveButton>`), so
/// this system's write set and the `With<ActiveButton>` set
/// [`paint_active_buttons`](crate::paint_active_buttons) writes are
/// DISJOINT — there is no write conflict between them.
///
/// Takes the theme as `Option<Res<GdtfTheme>>` so it is inert (rather than
/// panicking) before the resource is populated (bevy-traps rule 1). Registered by
/// [`UiPlugin`](crate::UiPlugin) in [`Update`] ordered
/// `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)` so
/// it composes on top of the freshest base look (bevy-traps rule 3).
pub fn repaint_deactivated_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut deactivated: RemovedComponents<ActiveButton>,
    mut buttons: Query<DeactivationVisuals, DeactivatedButton>,
) {
    let Some(theme) = theme else {
        return;
    };

    for entity in deactivated.read() {
        // Despawned / disabled / still-active entities miss this query and are
        // skipped (the `Err` arm) — see this system's doc-comment.
        let Ok((interaction, mut background, mut border)) = buttons.get_mut(entity) else {
            continue;
        };
        background.0 = interaction_fill(&theme, *interaction);
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}

/// Query filter selecting the buttons [`repaint_theme_change`] repaints on a theme
/// reload: enabled, NOT-active, NOT-segment, NOT-switch real buttons — exactly the
/// set whose fill is owned by the interaction layer
/// ([`theme_interaction`](super::theme_interaction)) rather than a special paint.
///
/// Factored into a named alias both to keep the system signature legible (clippy's
/// `type_complexity`) and to make the exclusions explicit. It mirrors
/// [`theme_interaction`](super::theme_interaction)'s filter *without* the
/// `Changed<Interaction>` term: this system's whole point is to repaint buttons
/// whose [`Interaction`](bevy::ui::Interaction) did NOT change at the reload frame,
/// so it must NOT gate on `Changed<Interaction>`. `Without<DisabledButton>` leaves
/// disabled buttons to
/// [`paint_disabled_buttons`](crate::paint_disabled_buttons),
/// `Without<ActiveButton>` leaves toggled-on buttons to
/// [`paint_active_buttons`](crate::paint_active_buttons) (the GTW-266
/// active-is-sticky rule), `Without<Segment>` leaves
/// [`SegmentedControl`](crate::SegmentedControl) segment fills to
/// [`repaint_segments`](crate::repaint_segments) (GTW-277), and
/// `Without<Switch>` leaves [`Switch`](crate::Switch) track fills to
/// [`drive_switches`](crate::drive_switches) (GTW-277). Those exclusions also make this
/// system's write set DISJOINT from each of those special paints, so there is no
/// `BackgroundColor` write conflict on a theme-change frame where they all run.
type EnabledInteractiveButton = (
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
    Without<Segment>,
    Without<Switch>,
);

/// Repaints every ENABLED interactive button to its CURRENT-[`Interaction`]-correct
/// fill the same frame [`GdtfTheme`] changes (GTW-147).
///
/// When the theme hot-reloads (the GTW-137 re-derive overwrites [`GdtfTheme`] in
/// place), [`apply_theme`](crate::themed::apply_theme) repaints every [`Themed`](crate::themed::Themed)
/// button to its ROLE/base fill from the new palette, IGNORING the button's current
/// [`Interaction`](bevy::ui::Interaction). The interaction feedback
/// ([`theme_interaction`](super::theme_interaction)) only fires on
/// `Changed<Interaction>`, so a button that is CURRENTLY `Hovered`/`Pressed` at the
/// reload frame — with no subsequent interaction change — was left showing the new
/// theme's resting base until the user moved off and re-hovered. This system closes
/// that gap: it re-derives each enabled button's fill from its CURRENT
/// [`Interaction`](bevy::ui::Interaction) via the SHARED [`interaction_fill`] helper
/// (so the `None`→base / `Hovered`→hover / `Pressed`→pressed mapping is never
/// duplicated, AC2), and re-affirms its [`BorderColor`](bevy::ui::BorderColor) from
/// the new button sub-theme.
///
/// ## What it does NOT touch (disjoint write set)
///
/// Disabled buttons keep the disabled fill [`paint_disabled_buttons`](crate::paint_disabled_buttons)
/// set (`Without<DisabledButton>`); active toggles keep the active fill
/// [`paint_active_buttons`](crate::paint_active_buttons) set
/// (`Without<ActiveButton>`); [`SegmentedControl`](crate::SegmentedControl)
/// segments (`Without<Segment>`) and [`Switch`](crate::Switch) tracks
/// (`Without<Switch>`) keep their owner-painted fills. Those four exclusions mirror
/// [`theme_interaction`](super::theme_interaction)'s filter and make this system's
/// write set DISJOINT from each special paint, so no two systems write the same
/// button's [`BackgroundColor`](bevy::ui::BackgroundColor) on the reload frame.
///
/// ## No feedback loop
///
/// It writes ONLY [`BackgroundColor`](bevy::ui::BackgroundColor) and
/// [`BorderColor`](bevy::ui::BorderColor) — it never touches
/// [`Interaction`](bevy::ui::Interaction), so it cannot trigger a
/// `Changed<Interaction>` and re-arm itself or [`theme_interaction`](super::theme_interaction)
/// (bevy-traps rule 4/7 — no messages, no `&mut World`).
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] ordered
/// `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)` so it
/// runs the SAME frame, after [`apply_theme`](crate::themed::apply_theme) has written
/// the base fills (bevy-traps rule 3), and gated on
/// `resource_changed::<GdtfTheme>().and_then(resource_exists::<GdtfTheme>)` so it does
/// nothing on steady-state frames (where it would otherwise clobber per-frame hover
/// feedback) and is inert and panic-free before the theme is populated (bevy-traps
/// rule 1).
pub fn repaint_theme_change(
    theme: Res<GdtfTheme>,
    mut buttons: Query<DeactivationVisuals, EnabledInteractiveButton>,
) {
    for (interaction, mut background, mut border) in &mut buttons {
        background.0 = interaction_fill(&theme, *interaction);
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}
