//! The battle action-bar (GTW-228 / GTW-48 S9 / 222c): a themed `gdtf_ui` button
//! surface that writes the SAME 222a act-intent seam the keyboard surface writes —
//! buttons + keys are PARALLEL surfaces over ONE data seam, not two divergent mappings.

mod components;
mod plugin;
mod systems;

// GTW-275 layout overhaul — the bar ROOT marker is no longer re-exported: `set_world_viewport`
// now insets the world map by the BOTTOM BAR only (the action bar is an in-bar overlay, not a
// separate inset). The marker stays internal to this module (used by `spawn_action_bar` /
// `despawn_action_bar` via the `components::` path).
pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeActionBarScenePlugin;
// GTW-298 — the relocated-controls spawn seam. The Firemode / Aim / Stance constructors are
// carried to the battlescape neighborhood so the sibling weapon-panel module can spawn the
// relocated controls INTO the weapon cluster (the Firemode / Aim / Stance panels), reusing this
// module's button logic + the gdtf_battle_input intents. The press → intent router
// (`action_bar_button_intents`), the active-mark syncs, and the `rebuild_mode_buttons`
// visibility driver stay registered by the action-bar plugin and find the relocated buttons
// parent-agnostically by marker.
pub(in crate::scenes::running::game::battlescape) use systems::{
    spawn_aim_button, spawn_mode_panel, spawn_stance_panel,
};

// Test-support-only re-export of the per-act button markers (the GTW-145 convention,
// the menu-button-marker precedent): widened to `pub` under `test-support` so the
// external integration tests can name them through `crate::test_support`, and gated so
// the production binary build stays `unused`/`unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    };
}
