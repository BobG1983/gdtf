//! [`ProcgenStepperPlugin`] — the DEV-ONLY procgen load-time stepper's registration
//! (GTW-655, restructured for a runtime toggle in GTW-868).
//!
//! ## Availability is compiled in; ENGAGEMENT is a runtime toggle
//!
//! 1. **`dev_tools` cfg (availability).** This module (and every item in it) compiles ONLY
//!    under the opt-in `dev_tools` Cargo feature (see `super::super::mod`'s
//!    `#[cfg(feature = "dev_tools")] pub(crate) mod procgen_stepper;`) — a release artifact,
//!    and even a normal `dynamic_linking` build without `dev_tools`, never links
//!    `bevy_egui`.
//! 2. **The Options-screen toggle (engagement).** Under `dev_tools` the drive systems below
//!    register UNCONDITIONALLY, but they only take a battle over while
//!    [`ProcgenStepperActive`](super::gate::ProcgenStepperActive) is in the world. That
//!    resource is inserted and removed at runtime by the dev-only procgen-stepper toggle on
//!    the Options screen (`crate::states::running::options`), which defaults to OFF
//!    (GTW-868) — so a plain `cargo drun` reaches a battle with no overlay and no pause,
//!    exactly as a build without `dev_tools` does, and `cargo dtest` never stalls.
//!    Engagement is read at `OnEnter(BattleScapeState::Generation)`, so a flip takes effect
//!    for the NEXT battle generation.
//!
//! [`with_enabled(true)`](ProcgenStepperPlugin::with_enabled) seeds that marker at plugin
//! build, which is how a headless test reaches the engaged path without driving the UI.
//!
//! ## Wiring
//!
//! - When `enabled`, inserts [`ProcgenStepperActive`](super::gate::ProcgenStepperActive) —
//!   the marker
//!   [`battle_setup_runs_directly`](super::gate::battle_setup_runs_directly) gates
//!   `request_battle_setup` off on, making the normal path and the stepper path MUTUALLY
//!   EXCLUSIVE per `Generation` entry (see the marker's own doc).
//! - `OnEnter(BattleScapeState::Generation)`, gated on that same marker existing:
//!   [`engage_stepper`](super::drive::engage_stepper)
//!   starts a fresh [`StagedProcgen`] drive (resolving the SAME authored situation + seed
//!   `request_battle_setup` would). With the marker absent nothing engages and the normal
//!   path runs — the two run conditions are exact complements, read from one resource.
//! - `Update`, gated `in_state(Generation)`:
//!   [`advance_stepper_drive`](super::drive::advance_stepper_drive) applies the latched
//!   command or the Auto timer, then (ordered `.after`)
//!   [`finish_stepper_drive`](super::drive::finish_stepper_drive) writes the SAME
//!   `SetupBattleRequested` the normal path writes once the drive completes. Both are inert
//!   with no drive in flight (they read `Option<Res<StagedProcgen>>`).
//! - `OnExit(BattleScapeState::Generation)`:
//!   [`cleanup_stepper_drive`](super::drive::cleanup_stepper_drive) — a safety net clearing
//!   every stepper resource this span may still hold (inert when there is none).
//! - Outside `test-support`: [`EguiPrimaryContextPass`], gated on [`StagedProcgen`] existing:
//!   [`draw_stepper_panel`](super::ui::draw_stepper_panel) draws the Next/Auto/Skip panel.
//!   That `resource_exists::<StagedProcgen>` gate is what keeps the panel off screen while
//!   the toggle is off — no drive, no panel.
//!   The `EguiPlugin` add itself (and the primary-context binding it makes owed) belongs to
//!   the dev aggregate, `crate::dev::plugin` — one owner, because Bevy panics on a duplicate
//!   plugin add (GTW-864; bevy-traps #8 for the multipass-aware wiring).
//!   Skipped under `test-support`: the egui overlay needs a primary window, which the
//!   GTW-655 headless integration test's `no_renderer.rs`-style app has none of (mirroring
//!   the content-editor's own headless harness, which excludes its windowed `EguiPlugin`
//!   for the same reason). The real binary never enables `test-support`, so this never
//!   changes shipped wiring; the test instead asserts the drive reaches the SAME result as
//!   the normal path via the resources directly (bevy-traps #8: the egui draw itself is
//!   screenshot-QA territory).

use bevy::prelude::*;
#[cfg(not(feature = "test-support"))]
use bevy_egui::EguiPrimaryContextPass;
#[cfg(not(feature = "test-support"))]
use gdtf_battle_sim::procgen::StagedProcgen;

#[cfg(not(feature = "test-support"))]
use super::ui::draw_stepper_panel;
use super::{
    drive::{advance_stepper_drive, cleanup_stepper_drive, engage_stepper, finish_stepper_drive},
    gate::ProcgenStepperActive,
};
use crate::states::BattleScapeState;

crate::support_item! {
    /// The DEV-ONLY procgen load-time stepper plugin.
    struct ProcgenStepperPlugin {
        /// Whether the stepper starts out ENGAGED. The shipped wiring passes `false` — the
        /// Options screen's dev-only toggle owns engagement from there on, by inserting /
        /// removing [`ProcgenStepperActive`]. A headless test passes `true` to reach the
        /// engaged path without driving the UI.
        enabled: bool,
    }
}

impl ProcgenStepperPlugin {
    crate::support_item! {
        /// Construct the plugin with its starting engagement set to `enabled`.
        ///
        /// `with_enabled(false)` is the shipped wiring (`crate::dev::plugin`): the systems
        /// register, nothing engages, and the Options screen's toggle drives engagement from
        /// there. `with_enabled(true)` seeds [`ProcgenStepperActive`] at plugin build, which
        /// is how the headless procgen-stepper suite drives the ENGAGED path deterministically
        /// without a running Options screen.
        #[must_use]
        const fn with_enabled(enabled: bool) -> Self {
            Self { enabled }
        }
    }

    /// Whether this plugin instance engages the stepper on `build`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

impl Plugin for ProcgenStepperPlugin {
    fn build(&self, app: &mut App) {
        if self.enabled {
            info!("procgen-stepper: engaged at startup (dev)");
            // The marker `battle_sim::plugin`'s run condition gates `request_battle_setup`
            // off on. Normally the Options toggle owns it; this seeds it for a test.
            app.insert_resource(ProcgenStepperActive);
        }
        // Registered unconditionally (GTW-868): engagement is decided per `Generation` entry
        // by the marker, not once at plugin-build time.
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
        // The egui overlay needs a primary window — skipped under `test-support` (see the
        // module doc); the real binary never enables that feature.
        #[cfg(not(feature = "test-support"))]
        app.add_systems(
            EguiPrimaryContextPass,
            draw_stepper_panel.run_if(resource_exists::<StagedProcgen>),
        );
    }
}

#[cfg(test)]
mod test {
    use super::ProcgenStepperPlugin;

    /// `with_enabled` records its flag verbatim — the same AC the auto-battle affordance
    /// pins for its own forced-enable constructor.
    #[test]
    fn with_enabled_records_the_flag_verbatim() {
        assert!(ProcgenStepperPlugin::with_enabled(true).enabled());
        assert!(!ProcgenStepperPlugin::with_enabled(false).enabled());
    }
}
