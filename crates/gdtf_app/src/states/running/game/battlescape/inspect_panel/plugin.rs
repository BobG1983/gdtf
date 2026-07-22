//! The inspect-panel scene-plugin (GTW-274).
//!
//! Registers the battle-scoped inspect panel in the battlescape neighborhood, beside the
//! status panel + action bar. It mirrors the status panel's lifecycle and gating:
//!
//! - **Lifecycle** — `spawn_inspect_panel` `OnEnter(BattleScapeState::BattleRunning)`,
//!   `despawn_inspect_panel` `OnExit(BattleScapeState::BattleRunning)`.
//! - **Update** — `update_inspect_panel` in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)` (`bevy-traps.md` #1), ordered
//!   `.after(InputSystems::Gather)` so it observes the SAME update's `pick_hovered_cell`
//!   write to `InspectTarget` (the GTW-264 ordering rationale — read the hover after it is
//!   gathered, not the previous frame's), AND ordered after the GTW-762 shadow promotes so
//!   it reads a cursor-time snapshot settled this frame (see below).
//!
//! # Cursor-time shadow ordering (GTW-762)
//!
//! The panel reads the cursor-time SHADOWS of the sim's grid / ledger / fog, not the live
//! resources, so it describes the cursor's playback position during closed-gate playback.
//! `promote_shown_occupancy` / `promote_shown_cover` refresh the two app-owned shadows, and
//! the presenter's `promote_shown_fog` (in `PresenterSystems::Compose`) refreshes the shared
//! fog shadow. `update_inspect_panel` is ordered `.after` all three — the two local systems
//! by name and the presenter promote by `.after(PresenterSystems::Compose)` — so the panel
//! never reads a shadow before it settled this frame. The two local promotes are ordered
//! `.after(PresenterSystems::Compose)` themselves, so their `is_changed` read of the grid
//! observes the fully-settled sim state (the whole `Draw` band runs after `SimSystems::Record`).
//!
//! Pure view: it reads the input crate's `InspectTarget` + the shadow snapshots of the sim's
//! `OccupancyGrid` / `CoverLedger` / `SquadVisibility` + on-entity vital components, owns no
//! combat rule, and writes no sim/input.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::inspect_panel::{
        shadow::{
            ShownCoverLedger, ShownOccupancyGrid, promote_shown_cover, promote_shown_occupancy,
        },
        systems::{despawn_inspect_panel, spawn_inspect_panel, update_inspect_panel},
    },
};

/// The inspect-panel scene-plugin — spawns/despawns the panel on the `BattleRunning` boundary
/// and runs its repaint system gated on the live-battle witness.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeInspectPanelScenePlugin;

impl Plugin for GameBattleScapeInspectPanelScenePlugin {
    fn build(&self, app: &mut App) {
        // GTW-762: the two app-owned cursor-time shadows always exist (empty Default =
        // fail-safe until the first promote), so `InspectReads` reads them as `Option`
        // only for a harness that never registered them.
        app.init_resource::<ShownOccupancyGrid>()
            .init_resource::<ShownCoverLedger>()
            .add_systems(
                OnEnter(BattleScapeState::BattleRunning),
                spawn_inspect_panel,
            )
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_inspect_panel,
            )
            .add_systems(
                Update,
                (promote_shown_occupancy, promote_shown_cover)
                    .after(PresenterSystems::Compose)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                update_inspect_panel
                    .after(InputSystems::Gather)
                    .after(PresenterSystems::Compose)
                    .after(promote_shown_occupancy)
                    .after(promote_shown_cover)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
