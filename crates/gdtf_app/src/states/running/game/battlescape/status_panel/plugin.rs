//! The status-panel scene-plugin (GTW-252).
//!
//! Registers the battle-scoped status HUD panel in the battlescape neighborhood,
//! beside the action-bar + presenter + input plugins. The panel shows the selected
//! player ganger's vitals — UI/view only (no sim/input change, no new act).
//!
//! - **Lifecycle** (mirrors the sibling action-bar) — `spawn_status_panel`
//!   `OnEnter(BattleScapeState::BattleRunning)`, `despawn_status_panel`
//!   `OnExit(BattleScapeState::BattleRunning)`, so the panel exists only during the
//!   live tactical layer (NOT the whole `GameState::BattleScape`, which spans the
//!   `Generation → AnimateIn → BattleRunning → AnimateOut → AfterMath` walk).
//! - **Update** — `update_status_panel` runs in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)`, the SAME live-battle witness the
//!   action-bar / input / presenter gate on (`bevy-traps.md` #1), so it is inert when
//!   no battle is live. It reads `Res<SelectedShooter>` (the input crate's selection
//!   resource) and repaints the lines from the selected ganger's vital components. It is
//!   ordered `.after(InputSystems::Gather)` (GTW-264) so it observes the SAME update's
//!   `auto_select_first_player_ganger` write to `SelectedShooter` — otherwise it read the
//!   selection before auto-select filled it and painted the empty state every frame
//!   (`bevy-traps.md` #3).
//!
//! The panel reads only DATA — the `Res<SelectedShooter>` selection and the on-entity
//! sim vital components — never a reverse edge into the sim/input or a cross-crate fn
//! (ADR-0001). It is a pure view; it owns no combat rule.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::status_panel::{
        stability_readout::update_stability_readout,
        systems::{despawn_status_panel, spawn_status_panel, update_status_panel},
    },
};

/// The status-panel scene-plugin — spawns/despawns the panel on the `BattleRunning`
/// boundary and runs its repaint system gated on the live-battle witness.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeStatusPanelScenePlugin;

impl Plugin for GameBattleScapeStatusPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_status_panel)
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_status_panel,
            )
            .add_systems(
                Update,
                (update_status_panel, update_stability_readout)
                    // GTW-264 — run AFTER the input crate's `InputSystems::Gather` band, where
                    // the GTW-255 `auto_select_first_player_ganger` writes the initial
                    // `SelectedShooter`. Without this ordering the panel read `SelectedShooter`
                    // BEFORE auto-select filled it and painted the empty "no ganger selected"
                    // state every frame (`bevy-traps.md` #3). The GTW-345 stability readout
                    // joins the same band — it reads the SAME `SelectedShooter` and repaints
                    // its bar beside the stat block.
                    .after(InputSystems::Gather)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
