//! The battle action-bar (GTW-228 / GTW-48 S9 / 222c): a themed `gdtf_ui` button
//! surface that writes the SAME 222a act-intent seam the keyboard surface writes —
//! buttons + keys are PARALLEL surfaces over ONE data seam, not two divergent mappings.

mod components;
mod plugin;
mod systems;

pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeActionBarScenePlugin;

// Test-support-only re-export of the per-act button markers (the GTW-145 convention,
// the menu-button-marker precedent): widened to `pub` under `test-support` so the
// external integration tests can name them through `crate::test_support`, and gated so
// the production binary build stays `unused`/`unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        AimToggleButton, EndTurnButton, FireModePickerButton, FireModePickerEntry,
        FireModePickerRoot, FireModePickerScrim, FleeButton, LevelDownButton, LevelUpButton,
        ReloadButton, StanceCycleButton,
    };
}
