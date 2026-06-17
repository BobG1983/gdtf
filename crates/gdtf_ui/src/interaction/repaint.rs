//! The deactivation-repaint system (GTW-280): repaint a just-de-selected toggle
//! the SAME frame it loses [`ActiveButton`], without waiting for a hover.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use super::theme::interaction_fill;
use crate::{
    theme::GdtfTheme,
    widgets::{ActiveButton, DisabledButton},
};

/// Query filter selecting the buttons [`repaint_deactivated_buttons`] repaints:
/// enabled, NOT-active buttons (so they are no longer painted by
/// [`paint_active_buttons`](crate::widgets::paint_active_buttons)).
///
/// Factored into a named alias both to keep the system signature legible
/// (clippy's `type_complexity`) and to make the exclusions explicit:
/// `With<Button>` restricts to real buttons, `Without<DisabledButton>` leaves
/// disabled buttons to [`paint_disabled_buttons`](crate::widgets::paint_disabled_buttons),
/// and `Without<ActiveButton>` confirms the button is no longer active (the
/// `RemovedComponents` signal fires the frame the marker is removed, by which
/// time the component is already gone). A despawned entity also surfaces in
/// [`RemovedComponents`](bevy::prelude::RemovedComponents) but misses this query,
/// so it is silently skipped.
type DeactivatedButton = (With<Button>, Without<DisabledButton>, Without<ActiveButton>);

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
/// Reads [`RemovedComponents`](bevy::prelude::RemovedComponents)`<`[`ActiveButton`](crate::widgets::ActiveButton)`>`
/// and, for each just-deactivated entity that is STILL present and matches
/// `(With<Button>, Without<DisabledButton>, Without<ActiveButton>)`, writes its
/// [`BackgroundColor`](bevy::ui::BackgroundColor) and re-affirms its
/// [`BorderColor`](bevy::ui::BorderColor) from its current
/// [`Interaction`](bevy::ui::Interaction) using the SAME button-state → fill
/// mapping as [`theme_interaction`](crate::interaction::theme_interaction) — the
/// shared [`interaction_fill`] helper, so the mapping is never duplicated (AC2).
///
/// ## Why this system exists
///
/// [`paint_active_buttons`](crate::widgets::paint_active_buttons) writes only
/// buttons `With<ActiveButton>`, and
/// [`theme_interaction`](crate::interaction::theme_interaction) runs only on
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
/// [`paint_disabled_buttons`](crate::widgets::paint_disabled_buttons)
/// (`Without<DisabledButton>`). A button that is somehow STILL active (e.g.
/// removed-then-re-added the same frame) is skipped (`Without<ActiveButton>`), so
/// this system's write set and the `With<ActiveButton>` set
/// [`paint_active_buttons`](crate::widgets::paint_active_buttons) writes are
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
