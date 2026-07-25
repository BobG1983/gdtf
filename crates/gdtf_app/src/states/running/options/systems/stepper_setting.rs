//! The DEV-ONLY procgen-stepper toggle's adapter, apply, view and ENGAGEMENT systems
//! (GTW-868). Compiled only under `dev_tools`.
//!
//! It is the sound toggle's shape, one step longer — because this setting has a world
//! effect beyond the readout:
//!
//! 1. [`stepper_activated`] — the `On<ValueChange<bool>>` observer that, for the
//!    procgen-stepper toggle (identified by its [`ProcgenStepperToggle`] marker), reads the
//!    first-party checkbox's native new value and writes the typed newtype
//!    [`ProcgenStepperSettingChanged`] intent (no bare `bool` crosses the boundary).
//! 2. [`apply_stepper_setting`] — folds that intent into the persisted [`GameSettings`].
//! 3. [`sync_stepper_engagement`] — inserts or removes
//!    [`ProcgenStepperActive`](crate::dev::procgen_stepper::ProcgenStepperActive), the ONE
//!    resource the stepper's `OnEnter(BattleScapeState::Generation)` run condition reads.
//!    Engagement is therefore decided per battle generation, at runtime: a flip takes effect
//!    for the NEXT generation entry (flipping mid-generation is out of scope). Message-driven
//!    rather than `resource_changed::<GameSettings>`-driven so it acts only on a real flip
//!    and never fights a test that seeded engagement at plugin build.
//! 4. [`sync_stepper_value_label`] — on a [`GameSettings`] change, MUTATES the value readout
//!    text in place (the project's mutate-not-respawn UI convention).
//! 5. [`paint_stepper_toggle`] — repaints the pill from the theme + setting, through the
//!    SAME shared [`repaint_toggle`] body the sound toggle uses.

use bevy::{prelude::*, ui::BackgroundColor, ui_widgets::ValueChange};
use gdtf_ui::theme::GdtfTheme;

use crate::{
    dev::procgen_stepper::ProcgenStepperActive,
    states::running::options::{
        components::{ProcgenStepperToggle, ProcgenStepperToggleKnob, ProcgenStepperValueLabel},
        settings::{
            GameSettings, ProcgenStepperEnabled, ProcgenStepperSettingChanged, stepper_value_text,
        },
        systems::theming::{ToggleVisuals, repaint_toggle, toggle_colors},
    },
};

/// The `On<ValueChange<bool>>` observer that writes the typed
/// [`ProcgenStepperSettingChanged`] intent when the dev-only procgen-stepper toggle is
/// activated.
///
/// A global observer (registered once at plugin build): it fires for every
/// [`ValueChange<bool>`](bevy::ui_widgets::ValueChange), so it filters on the
/// [`ProcgenStepperToggle`] marker and ignores the sound toggle's own change.
pub(in crate::states::running::options) fn stepper_activated(
    change: On<ValueChange<bool>>,
    toggles: Query<(), With<ProcgenStepperToggle>>,
    mut changed: MessageWriter<ProcgenStepperSettingChanged>,
) {
    if toggles.contains(change.source) {
        changed.write(ProcgenStepperSettingChanged::new(
            ProcgenStepperEnabled::new(change.value),
        ));
    }
}

/// Folds a [`ProcgenStepperSettingChanged`] intent into the persisted [`GameSettings`].
///
/// Applies the last-written value (a later intent this frame wins), mutating the resource
/// in place so the setting survives leaving and re-entering the screen.
pub(in crate::states::running::options) fn apply_stepper_setting(
    mut changed: MessageReader<ProcgenStepperSettingChanged>,
    mut settings: ResMut<GameSettings>,
) {
    for change in changed.read() {
        settings.procgen_stepper = **change;
    }
}

/// Inserts or removes
/// [`ProcgenStepperActive`](crate::dev::procgen_stepper::ProcgenStepperActive) to match a
/// flip of the procgen-stepper setting — the ONE place engagement is driven from at runtime.
///
/// Reads the same [`ProcgenStepperSettingChanged`] intent
/// [`apply_stepper_setting`] folds, so it acts exactly on a real flip and never on a merely
/// "changed" [`GameSettings`] (which would also fire on the resource's first frame and could
/// stomp an engagement a headless test seeded at plugin build). The stepper's own
/// `OnEnter(BattleScapeState::Generation)` run condition reads the resource, so the flip
/// takes effect for the NEXT battle generation.
pub(in crate::states::running::options) fn sync_stepper_engagement(
    mut changed: MessageReader<ProcgenStepperSettingChanged>,
    mut commands: Commands,
) {
    for change in changed.read() {
        if change.is_on() {
            commands.insert_resource(ProcgenStepperActive);
        } else {
            commands.remove_resource::<ProcgenStepperActive>();
        }
    }
}

/// Mutates the procgen-stepper value readout text when [`GameSettings`] changes.
///
/// Change-driven (`Res::is_changed`): on the frame the setting flips it rewrites the
/// [`ProcgenStepperValueLabel`] text ("On" / "Off") in place — no despawn/respawn.
pub(in crate::states::running::options) fn sync_stepper_value_label(
    settings: Res<GameSettings>,
    mut labels: Query<&mut Text, With<ProcgenStepperValueLabel>>,
) {
    if !settings.is_changed() {
        return;
    }
    let text = stepper_value_text(settings.procgen_stepper);
    for mut label in &mut labels {
        *label = Text::new(text);
    }
}

/// Repaints the procgen-stepper toggle from the current theme + setting, MUTATING its
/// painted fills and knob position in place — the same shared body the sound toggle uses.
///
/// Guarded `Option<Res<GdtfTheme>>` so it is inert before the theme is populated
/// (bevy-traps rule 1).
pub(in crate::states::running::options) fn paint_stepper_toggle(
    theme: Option<Res<GdtfTheme>>,
    settings: Res<GameSettings>,
    mut toggles: Query<ToggleVisuals, With<ProcgenStepperToggle>>,
    mut knobs: Query<
        &mut BackgroundColor,
        (
            With<ProcgenStepperToggleKnob>,
            Without<ProcgenStepperToggle>,
        ),
    >,
) {
    let Some(theme) = theme else {
        return;
    };
    let colors = toggle_colors(&theme);
    repaint_toggle(colors, settings.procgen_stepper, &mut toggles, &mut knobs);
}
