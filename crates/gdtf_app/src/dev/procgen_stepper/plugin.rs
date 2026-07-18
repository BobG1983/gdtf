//! [`ProcgenStepperPlugin`] — the DEV-ONLY procgen load-time stepper's registration seam
//! (GTW-655).
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **`dev_tools` cfg.** This module (and every item in it) compiles ONLY under the
//!    opt-in `dev_tools` Cargo feature (see `super::super::mod`'s `#[cfg(feature =
//!    "dev_tools")] pub(crate) mod procgen_stepper;`) — a release artifact, and even a
//!    normal `dynamic_linking` build without `dev_tools`, never links `bevy_egui`.
//! 2. **Opt-in env var.** Even in a `dev_tools` build the stepper is INERT by default: a
//!    plain `cargo run --features dev_tools` reaches a battle exactly as before. It
//!    activates only when `GDTF_PROCGEN_STEPPER` is set truthy
//!    ([`stepper_enabled`](super::gate::stepper_enabled)).
//!
//! ## Wiring when active
//!
//! - Inserts [`ProcgenStepperActive`](super::gate::ProcgenStepperActive) — a ONE-TIME,
//!   build-time marker (never per-`Generation`-span) that
//!   [`battle_setup_runs_directly`](super::gate::battle_setup_runs_directly) gates
//!   `request_battle_setup` off on, making the normal path and the stepper path MUTUALLY
//!   EXCLUSIVE per `Generation` entry with no ordering hazard between the two independent
//!   registration sites (see the marker's own doc).
//! - `OnEnter(BattleScapeState::Generation)`: [`engage_stepper`](super::drive::engage_stepper)
//!   starts a fresh [`StagedProcgen`] drive (resolving the SAME authored situation + seed
//!   `request_battle_setup` would).
//! - `Update`, gated `in_state(Generation)`:
//!   [`advance_stepper_drive`](super::drive::advance_stepper_drive) applies the latched
//!   command or the Auto timer, then (ordered `.after`)
//!   [`finish_stepper_drive`](super::drive::finish_stepper_drive) writes the SAME
//!   `SetupBattleRequested` the normal path writes once the drive completes.
//! - `OnExit(BattleScapeState::Generation)`:
//!   [`cleanup_stepper_drive`](super::drive::cleanup_stepper_drive) — a safety net clearing
//!   every stepper resource this span may still hold.
//! - Outside `test-support`: adds [`bevy_egui::EguiPlugin::default()`] — the recommended
//!   multipass-aware wiring (bevy-traps #8; this is the FIRST `gdtf_app` consumer of egui)
//!   — and [`EguiPrimaryContextPass`], gated on [`StagedProcgen`] existing:
//!   [`draw_stepper_panel`](super::ui::draw_stepper_panel) draws the Next/Auto/Skip panel.
//!   Skipped under `test-support`: the egui overlay needs a primary window, which the
//!   GTW-655 headless integration test's `no_renderer.rs`-style app has none of (mirroring
//!   the content-editor's own headless harness, which excludes its windowed `EguiPlugin`
//!   for the same reason). The real binary never enables `test-support`, so this never
//!   changes shipped wiring; the test instead asserts the drive reaches the SAME result as
//!   the normal path via the resources directly (bevy-traps #8: the egui draw itself is
//!   screenshot-QA territory).

use bevy::prelude::*;
#[cfg(not(feature = "test-support"))]
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
#[cfg(not(feature = "test-support"))]
use gdtf_battle_sim::procgen::StagedProcgen;

#[cfg(not(feature = "test-support"))]
use super::ui::draw_stepper_panel;
use super::{
    drive::{advance_stepper_drive, cleanup_stepper_drive, engage_stepper, finish_stepper_drive},
    gate::{ProcgenStepperActive, stepper_enabled},
};
use crate::states::BattleScapeState;

crate::support_item! {
    /// The DEV-ONLY procgen load-time stepper plugin.
    struct ProcgenStepperPlugin {
        /// Whether the affordance should activate. Captured once at construction (from
        /// [`stepper_enabled`] at the wiring site, or forced for a headless test of the
        /// drive logic) so `build` is a pure function of this flag.
        enabled: bool,
    }
}

impl ProcgenStepperPlugin {
    crate::support_item! {
        /// Construct the plugin, reading its env-var gate ([`stepper_enabled`]).
        #[must_use]
        fn from_env() -> Self {
            Self {
                enabled: stepper_enabled(),
            }
        }
    }

    /// Construct the plugin with its activation forced to `enabled`, bypassing the env-var
    /// read.
    ///
    /// The GTW-655 integration test uses this to drive BOTH gate branches deterministically
    /// (without racing a process-global env var) — `with_enabled(true)` proves the stepper
    /// path reaches `BattleRunning` with a result matching the normal path for the same
    /// seed, and `with_enabled(false)` (indistinguishable from no plugin) proves the normal
    /// path is unaffected by the wiring change.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn with_enabled(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Whether this plugin instance will activate on `build`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for ProcgenStepperPlugin {
    /// The wiring default: read the env-var gate.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for ProcgenStepperPlugin {
    fn build(&self, app: &mut App) {
        if !self.enabled {
            // Inert: register nothing. A battle loads exactly as it does without
            // `dev_tools`.
            return;
        }
        info!("procgen-stepper: ON (dev)");
        // The ONE-TIME, build-time marker `battle_sim::plugin`'s run condition gates
        // `request_battle_setup` off on (see the marker's own doc for why this avoids an
        // ordering hazard between the two independent registration sites).
        app.insert_resource(ProcgenStepperActive);
        app.add_systems(OnEnter(BattleScapeState::Generation), engage_stepper);
        app.add_systems(
            Update,
            (
                advance_stepper_drive,
                finish_stepper_drive.after(advance_stepper_drive),
            )
                .run_if(in_state(BattleScapeState::Generation)),
        );
        app.add_systems(OnExit(BattleScapeState::Generation), cleanup_stepper_drive);
        // The egui overlay needs a primary window — skipped under `test-support` (see the
        // module doc); the real binary never enables that feature.
        #[cfg(not(feature = "test-support"))]
        {
            app.add_plugins(EguiPlugin::default());
            app.add_systems(
                EguiPrimaryContextPass,
                draw_stepper_panel.run_if(resource_exists::<StagedProcgen>),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::ProcgenStepperPlugin;

    /// `with_enabled` records its flag verbatim and `from_env` agrees with the gate — the
    /// same AC the auto-battle affordance pins for its own forced-enable constructor.
    #[test]
    fn with_enabled_records_the_flag_verbatim() {
        assert!(ProcgenStepperPlugin::with_enabled(true).enabled());
        assert!(!ProcgenStepperPlugin::with_enabled(false).enabled());
    }
}
