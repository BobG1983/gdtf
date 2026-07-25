//! The DEV-ONLY procgen-stepper setting row (GTW-868) — compiled only under `dev_tools`.
//!
//! It is deliberately NOT a special control: it is the same [`spawn_setting_row`] every
//! other setting uses, so it gets the same themed pill toggle, the same value readout, and
//! (through the caller's one navigation chain) the same
//! [`DirectionalNavigationMap`](bevy::input_focus::directional_navigation::DirectionalNavigationMap)
//! edges and focusability the sound toggle and the Continue button have. The generic
//! "activate the focused control" path therefore drives it with no extra work.

use bevy::prelude::*;
use gdtf_ui::theme::GdtfTheme;

use super::row::{SettingCaption, SettingRow, SettingRowSpec, SettingValueText, spawn_setting_row};
use crate::states::running::options::{
    components::{ProcgenStepperToggle, ProcgenStepperToggleKnob, ProcgenStepperValueLabel},
    settings::{GameSettings, stepper_value_text},
};

/// Spawns the dev-only "Procgen Stepper" row and returns its entities.
///
/// Seeded from [`GameSettings::procgen_stepper`], which defaults OFF — so the row shows
/// "Off" the first time the screen is entered and the stepper stays disengaged until a
/// developer flips it.
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
