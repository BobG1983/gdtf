//! [`LoadingScreenPlugin`] — the GTW-419 battlescape loading-screen view.
//!
//! Registered by the
//! [`GameBattleScapeGenerationScenePlugin`](super::super::GameBattleScapeGenerationScenePlugin):
//! it spawns the full-viewport loading overlay `OnEnter(BattleScapeState::Generation)` and lets
//! [`DespawnOnExit`](bevy::state::prelude::DespawnOnExit)`(Generation)` (set on the root at
//! spawn) tear it down exactly on the transition to `AnimateIn`. The `Generation → AnimateIn`
//! transition itself stays UNCHANGED — gated on the sim's `BattleReady` signal via the existing
//! `GenerationComplete`-keyed `move_on` — so the loading screen covers the WHOLE Generation phase
//! and is removed the instant the assembled level appears. It is a presenter / view artifact; it
//! owns no combat rules and never touches the sim.

use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::spawn::spawn_loading_screen,
};

/// Spawns + (via `DespawnOnExit`) tears down the battlescape loading screen across the
/// Generation phase.
///
/// Wiring (all additive — it touches no sim / camera / input):
///
/// - `OnEnter(BattleScapeState::Generation)`: [`spawn_loading_screen`] builds the opaque
///   full-viewport overlay (marker [`LoadingScreenRoot`](super::components::LoadingScreenRoot),
///   high `GlobalZIndex`, `DespawnOnExit(Generation)`).
/// - The root's `DespawnOnExit(Generation)` despawns the whole subtree on `OnExit(Generation)`
///   (the transition to `AnimateIn`) — no explicit cleanup system needed (`bevy-traps.md` #1,
///   applied to entities).
/// - Under `cfg!(all(debug_assertions, feature = "net_qa"))` ONLY, the env-gated
///   loading-screen capture hook ([`register_loading_capture`](super::capture::register_loading_capture))
///   is also registered — inert unless `GDTF_LOADING_SHOT` is set (the QA hook for AC2 / C2).
pub(in crate::states::running::game::battlescape::generation) struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::Generation), spawn_loading_screen);

        // DEV-ONLY QA hook: the loading-screen self-screenshot, double-gated on the dev cfg + its
        // own env var (the GTW-297 capture discipline). Inert in a normal build.
        #[cfg(all(debug_assertions, feature = "net_qa"))]
        super::capture::register_loading_capture(app);
    }
}
