//! The GTW-141 mouse-hover-follows-focus bridge.

use bevy::{
    input_focus::InputFocus,
    prelude::*,
    ui::{Interaction, widget::Button},
};

use crate::widgets::DisabledButton;

/// Query filter selecting the buttons [`sync_hover_to_focus`] reacts to: enabled
/// buttons whose [`Interaction`](bevy::ui::Interaction) changed this frame.
///
/// A named alias so the system signature stays legible (clippy's
/// `type_complexity`) and the exclusion is explicit: `Without<DisabledButton>`
/// is what keeps a disabled button from ever stealing focus on hover (AC#2), and
/// `Changed<Interaction>` limits the work to the frame the hover transition
/// lands rather than re-asserting focus every frame a button stays hovered.
type HoverableButton = (Changed<Interaction>, With<Button>, Without<DisabledButton>);

/// Moves input focus onto whatever enabled button the mouse is hovering.
///
/// For every [`Button`](bevy::ui::Button) whose
/// [`Interaction`](bevy::ui::Interaction) became
/// [`Hovered`](bevy::ui::Interaction::Hovered) this frame — and which is **not** a
/// [`DisabledButton`](crate::widgets::DisabledButton) — it points the
/// [`InputFocus`](bevy::input_focus::InputFocus) resource at that entity. The
/// effect is "focus follows the mouse": a pointer hover lands the same focus a
/// keyboard / gamepad navigation would, so the two input modes stay on one
/// shared cursor and a subsequent `Enter` / gamepad-South activates the button
/// the mouse last touched.
///
/// The [`Interaction`](bevy::ui::Interaction) it reads is written upstream by
/// `bevy_ui`'s built-in `ui_focus_system` (in `PreUpdate`, from raw mouse input)
/// — this system adds **no** click or hover detection of its own; it only
/// bridges the already-resolved hover onto [`InputFocus`](bevy::input_focus::InputFocus).
///
/// It composes with GTW-119's directional navigation: both update
/// [`InputFocus`](bevy::input_focus::InputFocus), but they do not fight — this
/// one only fires on a `Changed<Interaction>` hover transition and that one only
/// on a [`NavigateRequest`](crate::focus_nav::NavigateRequest), so each writes
/// only in response to its own distinct signal.
///
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`] ordered
/// `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)`
/// — the same band as [`theme_interaction`](super::theme_interaction) (bevy-traps
/// rule 3). [`InputFocus`](bevy::input_focus::InputFocus) is initialized by the
/// `InputDispatchPlugin` that [`UiPlugin`](crate::UiPlugin) installs via the
/// focus-nav layer, so it is always present and the `ResMut` never panics
/// (bevy-traps rule 1).
pub fn sync_hover_to_focus(
    mut focus: ResMut<InputFocus>,
    buttons: Query<(Entity, &Interaction), HoverableButton>,
) {
    for (entity, interaction) in &buttons {
        if *interaction == Interaction::Hovered {
            focus.set(entity);
        }
    }
}
