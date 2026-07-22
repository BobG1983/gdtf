//! [`BattleSimPlugin`] — the thin app-side glue that drives the SIM-OWNED
//! `gdtf_battle_sim::battle::BattleSimPlugin` across the battle lifecycle (E10.5 / GTW-207).
//!
//! It is the VIEW-side glue (`docs/decisions/0001-rust-bevy-rewrite.md`: the model is
//! the authoritative render-free sim, consumed ONE-WAY by the app). The sim owns its
//! own integration plugin and names NO app type; this app-side plugin adds that sim
//! plugin (one line) and the THIN trigger / gate / teardown systems that bridge the
//! app's state machine to the sim's lifecycle messages — it only SENDS triggers and
//! GATES on the sim's [`BattleReady`](gdtf_battle_sim::battle::BattleReady) signal,
//! never duplicating combat rules. Registered by the
//! [`GameBattleScapeGenerationScenePlugin`](super::super::GameBattleScapeGenerationScenePlugin)
//! tree; it adds no new Cargo edge (the `gdtf_app -> gdtf_battle_sim` path is E10.1's,
//! consumed here).

use bevy::prelude::*;
use gdtf_battle_sim::{battle::BattleSimPlugin as SimBattleSimPlugin, occupancy_sync::SimSystems};

use crate::states::{
    BattleScapeState, GameState,
    running::game::battlescape::generation::{
        battle_sim::systems::{
            gate_generation_complete, request_battle_setup, request_battle_teardown,
        },
        resources::GenerationComplete,
    },
};

/// Drives the sim-owned `gdtf_battle_sim::battle::BattleSimPlugin` across the app's battle
/// lifecycle.
///
/// Wiring (all additive — it touches no presenter / camera / window / input):
///
/// 1. Adds [`gdtf_battle_sim::battle::BattleSimPlugin`](SimBattleSimPlugin) — the ONE plugin
///    that wires the whole sim runtime (bundling `OccupancyMaintenancePlugin` +
///    `SimActsPlugin`, registering the three lifecycle messages, and adding the sim's
///    setup / teardown systems in `SimSystems::Simulate`).
/// 2. `OnEnter(BattleScapeState::Generation)`:
///    [`request_battle_setup`](super::systems::request_battle_setup) writes a
///    `SetupBattleRequested` carrying the authored situation + the placeholder seed.
///
///    GTW-655: under the `dev_tools` feature, this is additionally gated
///    `.run_if(`[`battle_setup_runs_directly`](crate::dev::procgen_stepper::battle_setup_runs_directly)`)`
///    — `true` (the UNCHANGED behavior below) whenever the dev-tools procgen stepper is
///    NOT enabled for this process. When it IS enabled, this system is skipped for every
///    `Generation` entry and `crate::dev::procgen_stepper`'s own `OnEnter(Generation)`
///    system drives the battle instead, one stage at a time, finishing through the SAME
///    `SetupBattleRequested` write. Without `dev_tools` compiled in at all, this line does
///    not exist and the registration below is identical to before GTW-655.
/// 3. `Update`, presence-gated and ordered `.after(SimSystems::Simulate)`:
///    [`gate_generation_complete`](super::systems::gate_generation_complete) inserts
///    [`GenerationComplete`] on the sim's `BattleReady` signal — strictly after the
///    sim's setup system has run THIS update (`bevy-traps.md` #3), so the signal
///    written this update is read this update (no one-frame lag), and the existing
///    `move_on` advances to `AnimateIn` only after a successful setup.
/// 4. `OnExit(GameState::BattleScape)`:
///    [`request_battle_teardown`](super::systems::request_battle_teardown) writes a
///    `TeardownBattleRequested` so the sim cleans the battle-lifetime resources at the
///    BATTLE boundary (NOT the Generation boundary) — they survive the whole battle
///    for the E10.6 acts (`bevy-traps.md` #1 at the correct state level).
pub(in crate::states::running::game::battlescape::generation) struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimBattleSimPlugin);
        // GTW-655: gate `request_battle_setup` off exactly while the dev-tools procgen
        // stepper is engaged for this process — see the doc above. Without `dev_tools`
        // compiled, this registration is UNCHANGED (no run_if at all), so a non-`dev_tools`
        // build behaves identically to before this ticket.
        #[cfg(feature = "dev_tools")]
        app.add_systems(
            OnEnter(BattleScapeState::Generation),
            request_battle_setup.run_if(crate::dev::procgen_stepper::battle_setup_runs_directly),
        );
        #[cfg(not(feature = "dev_tools"))]
        app.add_systems(OnEnter(BattleScapeState::Generation), request_battle_setup);
        app.add_systems(
            Update,
            gate_generation_complete.after(SimSystems::Simulate).run_if(
                in_state(BattleScapeState::Generation)
                    .and_then(not(resource_exists::<GenerationComplete>)),
            ),
        )
        .add_systems(OnExit(GameState::BattleScape), request_battle_teardown);
    }
}
