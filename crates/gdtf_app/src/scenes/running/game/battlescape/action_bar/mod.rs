//! The battle action-bar (GTW-228 / GTW-48 S9 / 222c): a themed `gdtf_ui` button
//! surface that writes the SAME 222a act-intent seam the keyboard surface writes —
//! buttons + keys are PARALLEL surfaces over ONE data seam, not two divergent mappings.

mod components;
mod plugin;
mod systems;

// GTW-271 — the bar ROOT marker, re-exported to the battlescape neighborhood so the
// `set_world_viewport` system can measure its `ComputedNode` height for the world-map's BOTTOM
// margin inset (no test-support gating — it stays inside the neighborhood).
pub(in crate::scenes::running::game::battlescape) use components::ActionBarRoot;
pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeActionBarScenePlugin;

// Test-support-only re-export of the per-act button markers (the GTW-145 convention,
// the menu-button-marker precedent): widened to `pub` under `test-support` so the
// external integration tests can name them through `crate::test_support`, and gated so
// the production binary build stays `unused`/`unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton, ReloadButton,
        StanceKneelingButton, StancePanelRoot, StanceProneButton, StanceStandingButton,
    };
}
