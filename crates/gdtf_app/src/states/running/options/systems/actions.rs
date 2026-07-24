//! The Continue button's activation → leaving the Options screen (GTW-637).
//!
//! The Options screen is a real interactive stop (it no longer auto-advances): the
//! player leaves it by activating the [`ContinueButton`], which requests
//! [`RunningState::Game`]. Per the ADR 0003 amendment (2026-07-05, GTW-635) this
//! pilot drives its input through the first-party `bevy_ui_widgets` observer
//! contract: activation is a [`bevy_ui_widgets::Activate`](Activate) event handled by
//! an `On<Activate>` observer, rather than a buffered message. Two pieces make that
//! up, mirroring how `bevy_ui_widgets`' own `ButtonPlugin` turns raw input into an
//! `Activate`:
//!
//! 1. [`bridge_continue_activation`] — a bridge system that translates the two input
//!    signals the project already produces ([`Interaction::Pressed`] from the built-in
//!    `ui_focus_system` on a mouse press, and a [`FocusActivated`] message from the
//!    focus-nav bridge on `Enter` / gamepad South) into a `commands.trigger(Activate
//!    { entity })` on the Continue button. The project does not enable Bevy's
//!    `ui_picking` backend (which would let the widgets read pointer events
//!    themselves), so this bridge is the honest adapter cost of driving a first-party
//!    widget from the existing, arbitration-proven input path — it changes nothing
//!    outside this screen, so the battle `cursor_over_ui` / act-bus arbitration cannot
//!    regress.
//! 2. [`continue_activated`] — the `On<Activate>` observer that requests
//!    [`RunningState::Game`] when the Continue button is activated. Registered as a
//!    global observer (once, at plugin build); it filters by the [`ContinueButton`]
//!    marker so a sound-toggle `Activate` (the other trigger on this screen) is a
//!    no-op for it.

use bevy::{prelude::*, ui::Interaction, ui_widgets::Activate};
use gdtf_ui::focus_nav::FocusActivated;

use crate::states::{RunningState, running::options::components::ContinueButton};

/// Query filter for a [`ContinueButton`] whose [`Interaction`](bevy::ui::Interaction)
/// changed this frame. Aliased out of the `type_complexity` deny lint.
type ContinuePressedFilter = (Changed<Interaction>, With<ContinueButton>);

/// Translates the project's pointer / keyboard input signals into a
/// [`bevy_ui_widgets::Activate`](Activate) trigger on the [`ContinueButton`].
///
/// Fires a `commands.trigger(Activate { entity })` when the Continue button's
/// [`Interaction`](bevy::ui::Interaction) changed to
/// [`Pressed`](bevy::ui::Interaction::Pressed) this frame (the mouse path;
/// `Changed<Interaction>` limits it to the landing frame), or when a
/// [`FocusActivated`] message names the button (the keyboard / gamepad path). The
/// triggered `Activate` is handled by [`continue_activated`]. Registered under
/// `run_if(in_state(RunningState::Options))` and ordered `.after(FocusNavSystems::Bridge)`
/// by the scene plugin so a same-frame `Enter` is caught the frame it is raised.
pub(in crate::states::running::options) fn bridge_continue_activation(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction), ContinuePressedFilter>,
    mut activations: MessageReader<FocusActivated>,
    buttons: Query<(), With<ContinueButton>>,
) {
    for (entity, interaction) in &pressed {
        if matches!(interaction, Interaction::Pressed) {
            commands.trigger(Activate { entity });
        }
    }
    for activated in activations.read() {
        if buttons.contains(**activated) {
            commands.trigger(Activate {
                entity: **activated,
            });
        }
    }
}

/// The `On<Activate>` observer that requests [`RunningState::Game`] when the
/// [`ContinueButton`] is activated.
///
/// A global observer (registered once at plugin build): it fires for every
/// [`Activate`] trigger, so it filters on the [`ContinueButton`] marker and ignores
/// an `Activate` on any other entity (the sound toggle's activation, on this screen).
/// This is the pilot's `On<Activate>` INPUT path — the first-party widget observer
/// contract, not a buffered message.
pub(in crate::states::running::options) fn continue_activated(
    activate: On<Activate>,
    buttons: Query<(), With<ContinueButton>>,
    mut next: ResMut<NextState<RunningState>>,
) {
    if buttons.contains(activate.entity) {
        next.set(RunningState::Game);
    }
}
