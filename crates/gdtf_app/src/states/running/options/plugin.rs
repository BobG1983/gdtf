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
        #[cfg(feature = "dev_tools")]
        add_dev_stepper_setting(app);
    }
}

#[cfg(feature = "dev_tools")]
fn add_dev_stepper_setting(app: &mut App) {
    use crate::states::running::options::{
        settings::ProcgenStepperSettingChanged,
        systems::{
            apply_stepper_setting, paint_stepper_toggle, stepper_activated,
            sync_stepper_engagement, sync_stepper_value_label,
        },
    };

    app.add_message::<ProcgenStepperSettingChanged>()
        .add_observer(stepper_activated);
    app.add_systems(
        Update,
        (
            apply_stepper_setting,
            sync_stepper_engagement.after(apply_stepper_setting),
            sync_stepper_value_label.after(apply_stepper_setting),
        )
            .run_if(in_state(RunningState::Options)),
    );
    app.add_systems(
        Update,
        paint_stepper_toggle.after(apply_stepper_setting).run_if(
            in_state(RunningState::Options)
                .and_then(resource_exists::<GdtfTheme>)
                .and_then(resource_changed::<GameSettings>.or_else(resource_changed::<GdtfTheme>)),
        ),
    );
}

fn add_observers(app: &mut App) {
    app.add_observer(continue_activated)
        .add_observer(sound_activated)
        .add_observer(checkbox_self_update);
}

fn add_settings(app: &mut App) {
    app.init_resource::<GameSettings>()
        .add_message::<SoundSettingChanged>();
}

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

    app.add_systems(
        Update,
        (
            bridge_continue_activation.after(FocusNavSystems::Bridge),
            apply_sound_setting,
            sync_sound_value_label.after(apply_sound_setting),
        )
            .run_if(in_state(RunningState::Options)),
    );

    app.add_systems(
        Update,
        paint_sound_toggle.after(apply_sound_setting).run_if(
            in_state(RunningState::Options)
                .and_then(resource_exists::<GdtfTheme>)
                .and_then(resource_changed::<GameSettings>.or_else(resource_changed::<GdtfTheme>)),
        ),
    );
}
