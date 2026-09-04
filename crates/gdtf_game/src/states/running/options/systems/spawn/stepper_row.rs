use bevy::prelude::*;
use gdtf_ui::theme::GdtfTheme;

use super::row::{SettingCaption, SettingRow, SettingRowSpec, SettingValueText, spawn_setting_row};
use crate::states::running::options::{
    components::{ProcgenStepperToggle, ProcgenStepperToggleKnob, ProcgenStepperValueLabel},
    settings::{GameSettings, stepper_value_text},
};

pub(super) fn spawn_stepper_row(
    commands: &mut Commands,
    theme: &GdtfTheme,
    settings: GameSettings,
) -> SettingRow {
    let stepper = settings.procgen_stepper;
    spawn_setting_row(
        commands,
        theme,
        SettingRowSpec {
            caption:       SettingCaption::new("Procgen Stepper (dev)"),
            value:         stepper,
            value_text:    SettingValueText::new(stepper_value_text(stepper)),
            toggle_marker: ProcgenStepperToggle,
            knob_marker:   ProcgenStepperToggleKnob,
            value_marker:  ProcgenStepperValueLabel,
        },
    )
}
