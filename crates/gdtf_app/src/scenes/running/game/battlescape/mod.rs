mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameBattleScapeScenePlugin;
mod resources;

mod generation;
pub(in crate::scenes::running::game::battlescape) use generation::GameBattleScapeGenerationScenePlugin;

mod animate_in;
pub(in crate::scenes::running::game::battlescape) use animate_in::GameBattleScapeAnimateInScenePlugin;

mod battle_running;
pub(in crate::scenes::running::game::battlescape) use battle_running::GameBattleScapeBattleRunningScenePlugin;
// Test-support-only re-export of the explicit end-signal marker (GTW-236), gated so the
// binary build stays `unused`/`unreachable_pub`-clean (the action-bar marker re-export chain
// precedent). Carries `BattleRunningComplete` up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use!(battle_running::BattleRunningComplete;);

mod animate_out;
pub(in crate::scenes::running::game::battlescape) use animate_out::GameBattleScapeAnimateOutScenePlugin;

mod aftermath;
pub(in crate::scenes::running::game::battlescape) use aftermath::GameBattleScapeAfterMathScenePlugin;

mod action_bar;
pub(in crate::scenes::running::game::battlescape) use action_bar::GameBattleScapeActionBarScenePlugin;
// Test-support-only re-export of the action-bar's per-act button markers (GTW-228),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the menu-marker
// precedent). The AC tests name these through `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    action_bar::{
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton, ReloadButton,
        StanceKneelingButton, StancePanelRoot, StanceProneButton, StanceStandingButton,
    };
}

mod status_panel;
pub(in crate::scenes::running::game::battlescape) use status_panel::GameBattleScapeStatusPanelScenePlugin;
// Test-support-only re-export of the status-panel's per-line text markers (GTW-252),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the action-bar
// per-act-marker precedent). The AC tests name these through `crate::test_support` to
// assert each line's `Text` content.
#[cfg(feature = "test-support")]
crate::support_use! {
    status_panel::{HpText, IdentityText, LifeText, StanceText, TuText, WeaponNameText};
}
