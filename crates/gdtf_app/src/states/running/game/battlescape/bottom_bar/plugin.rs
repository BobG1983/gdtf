//! The bottom-bar scene-plugin (GTW-275 layout overhaul, item 6).
//!
//! Registers the battle-scoped opaque bottom strip in the battlescape neighborhood, beside
//! the weapon-panel / action-bar / status-panel / hover-panel plugins. The bar is the ONLY UI
//! that reduces the world map (its measured height is the sole viewport inset — the corner
//! panels are overlays); it hosts the weapon panel + the out-of-scope controls / contextual
//! panels. View-only — it owns no sim/input state.
//!
//! - **Lifecycle** (mirrors the sibling weapon panel) — `spawn_bottom_bar`
//!   `OnEnter(BattleScapeState::BattleRunning)`, `despawn_bottom_bar`
//!   `OnExit(BattleScapeState::BattleRunning)`, so the bar exists only during the live
//!   tactical layer (NOT the whole `GameState::BattleScape`).
//!
//! `Update` (GTW-298): `opacify_bottom_bar` keeps the bar's panel fill OPAQUE and
//! `repad_bottom_bar` keeps its four-sided content padding, both `.after(UiSystems::ApplyTheme)`
//! (the theme paints the panel fill at `alpha 0.55`, which would bleed the map dots through — R3,
//! and `box_node` clobbers `Node::padding` with the px panel margin, removing the content's
//! breathing room), gated on the live-battle witness. Otherwise the bar is static;
//! `set_world_viewport` (the battlescape's own `Update` system) reads the bar's `ComputedNode` to
//! inset the map.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::themed::UiSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::bottom_bar::systems::{
        despawn_bottom_bar, opacify_bottom_bar, repad_bottom_bar, spawn_bottom_bar,
    },
};

/// The bottom-bar scene-plugin — spawns/despawns the opaque strip on the `BattleRunning`
/// boundary.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeBottomBarScenePlugin;

impl Plugin for GameBattleScapeBottomBarScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_bottom_bar)
            .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_bottom_bar)
            // GTW-298: keep the bottom strip an OPAQUE solid panel (the theme's `alpha 0.55`
            // panel fill bled the map dots through — R3) AND keep its four-sided content padding
            // (the theme's `box_node` clobbers `Node::padding` with the px panel margin, removing
            // the breathing room around the weapon cluster + stance column). Both ordered
            // `.after(UiSystems::ApplyTheme)` so they run after the theme pass repaints the
            // panel, then re-raise the fill alpha to 1.0 and re-apply the relative-unit padding
            // (each a no-op once settled). Same live-battle gate as the sibling weapon-panel /
            // action-bar fit passes.
            .add_systems(
                Update,
                (opacify_bottom_bar, repad_bottom_bar)
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
