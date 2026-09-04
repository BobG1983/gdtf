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

pub(in crate::states::running::options) fn apply_stepper_setting(
    mut changed: MessageReader<ProcgenStepperSettingChanged>,
    mut settings: ResMut<GameSettings>,
) {
    for change in changed.read() {
        settings.procgen_stepper = **change;
    }
}

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
