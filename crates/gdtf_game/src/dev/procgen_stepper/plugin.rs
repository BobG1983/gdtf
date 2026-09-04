//! Procgen stepper plugin (`dev_tools`).

use bevy::prelude::*;
#[cfg(not(feature = "headless_test"))]
use bevy_egui::EguiPrimaryContextPass;
#[cfg(not(feature = "headless_test"))]
use gdtf_battle_sim::procgen::StagedProcgen;

#[cfg(not(feature = "headless_test"))]
use super::ui::draw_stepper_panel;
use super::{
    drive::{advance_stepper_drive, cleanup_stepper_drive, engage_stepper, finish_stepper_drive},
    gate::ProcgenStepperActive,
};
use crate::states::BattleScapeState;

crate::support_item! {
    /// Drives situation generation one stage at a time.
    struct ProcgenStepperPlugin {
        enabled: bool,
    }
}

impl ProcgenStepperPlugin {
    crate::support_item! {
        /// Build the plugin with the stepper on or off.
        #[must_use]
        const fn with_enabled(enabled: bool) -> Self {
            Self { enabled }
        }
    }

    /// Whether the stepper is enabled.
    #[cfg(feature = "headless_test")]
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

impl Plugin for ProcgenStepperPlugin {
    fn build(&self, app: &mut App) {
        if self.enabled {
            info!("procgen-stepper: engaged at startup (dev)");
            app.insert_resource(ProcgenStepperActive);
        }
        app.add_systems(
            OnEnter(BattleScapeState::Generation),
            engage_stepper.run_if(resource_exists::<ProcgenStepperActive>),
        );
        app.add_systems(
            Update,
            (
                advance_stepper_drive,
                finish_stepper_drive.after(advance_stepper_drive),
            )
                .run_if(in_state(BattleScapeState::Generation)),
        );
        app.add_systems(OnExit(BattleScapeState::Generation), cleanup_stepper_drive);
        #[cfg(not(feature = "headless_test"))]
        app.add_systems(
            EguiPrimaryContextPass,
            draw_stepper_panel.run_if(resource_exists::<StagedProcgen>),
        );
    }
}

#[cfg(test)]
mod test {
    use super::ProcgenStepperPlugin;

    #[test]
    fn with_enabled_records_the_flag_verbatim() {
        assert!(ProcgenStepperPlugin::with_enabled(true).enabled());
        assert!(!ProcgenStepperPlugin::with_enabled(false).enabled());
    }
}
