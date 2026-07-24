//! The Options scene plugin (GTW-637): registers the settings model, spawns the
//! `bsn!`-first widget screen on entry, and wires its action / adapter / theming
//! systems.
//!
//! The screen replaces the pre-GTW-637 zero-UI stub that inserted a completion
//! marker and auto-advanced to `Game`. It is now a real interactive stop: the
//! player leaves it by activating the Continue button. The persisted
//! [`GameSettings`] resource is inserted here (app-wide, not scene-scoped) so a
//! setting survives leaving and re-entering the screen; the screen tree itself is
//! torn down by its `DespawnOnExit(RunningState::Options)` markers on leave.

use bevy::{prelude::*, ui_widgets::checkbox_self_update};
use gdtf_ui::{focus_nav::FocusNavSystems, theme::GdtfTheme};

use crate::states::{
    RunningState,
    running::options::{
        settings::{GameSettings, SoundSettingChanged},
        systems::{
            apply_sound_setting, bridge_continue_activation, clear_options_nav_map,
            continue_activated, paint_sound_toggle, sound_activated, spawn_options_screen,
            sync_sound_value_label,
        },
    },
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

pub(in crate::states) struct OptionsScenePlugin;

impl Plugin for OptionsScenePlugin {
    fn build(&self, app: &mut App) {
        add_settings(app);
        add_observers(app);
        add_systems(app);
    }
}

/// Registers the screen's activation observers (GTW-637 INPUT clause).
///
/// Global observers (registered once): [`continue_activated`] requests
/// [`RunningState::Game`] on the Continue button's
/// [`Activate`](bevy::ui_widgets::Activate), and [`sound_activated`] writes the typed
/// [`SoundSettingChanged`] intent on the sound toggle's native
/// [`ValueChange<bool>`](bevy::ui_widgets::ValueChange) — the first-party
/// [`Checkbox`](bevy::ui_widgets::Checkbox)'s own activation event, with no hand-rolled
/// bridge in between. Each filters by its own marker, so the two never cross.
/// [`checkbox_self_update`] is the first-party observer that keeps the checkbox's
/// [`Checked`](bevy::ui::Checked) state in step with each
/// [`ValueChange`](bevy::ui_widgets::ValueChange)
/// (it is NOT added by the `CheckboxPlugin`, so the screen opts into it here) — without
/// it the checkbox would report the same stale checked state on every activation and
/// never actually toggle.
fn add_observers(app: &mut App) {
    app.add_observer(continue_activated)
        .add_observer(sound_activated)
        .add_observer(checkbox_self_update);
}

/// Registers the app-wide settings model: the persisted [`GameSettings`] resource
/// and the typed [`SoundSettingChanged`] intent message.
fn add_settings(app: &mut App) {
    app.init_resource::<GameSettings>()
        .add_message::<SoundSettingChanged>();
}

/// Wires the Options scene's enter/exit and per-frame systems.
fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Running::Options");
    app.add_systems(
        OnEnter(RunningState::Options),
        (log_scene_enter(label), spawn_options_screen),
    )
    .add_systems(
        OnExit(RunningState::Options),
        (log_scene_exit(label), clear_options_nav_map),
    );

    // The Continue button's input bridge, the sound-toggle apply/view pipeline, and the
    // toggle repaint, all gated to the Options screen. `bridge_continue_activation` runs
    // after the focus-nav Bridge that writes `FocusActivated` so a same-frame Enter is
    // caught the frame it is raised. The sound toggle needs NO input bridge: its native
    // `ValueChange` observer (`sound_activated`) writes `SoundSettingChanged` directly;
    // `apply_sound_setting` folds that intent into `GameSettings`, and
    // `sync_sound_value_label` / `paint_sound_toggle` repaint the readout and the toggle
    // in place, ordered after the fold so a flip lands without a stale frame
    // (bevy-traps rule 3).
    app.add_systems(
        Update,
        (
            bridge_continue_activation.after(FocusNavSystems::Bridge),
            apply_sound_setting,
            sync_sound_value_label.after(apply_sound_setting),
        )
            .run_if(in_state(RunningState::Options)),
    );

    // Repaint the headless checkbox toggle in place whenever the setting flips (the
    // toggle) OR the theme changes (the GTW-137 hot-reload) — the same body serves
    // both. Gated on the screen being up and a change to either resource, ordered after
    // the setting fold so a flip's new value is painted the same frame.
    app.add_systems(
        Update,
        paint_sound_toggle.after(apply_sound_setting).run_if(
            in_state(RunningState::Options)
                .and_then(resource_exists::<GdtfTheme>)
                .and_then(resource_changed::<GameSettings>.or_else(resource_changed::<GdtfTheme>)),
        ),
    );
}
