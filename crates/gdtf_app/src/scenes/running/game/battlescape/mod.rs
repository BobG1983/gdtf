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
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    };
}

// The GTW-278 shared ganger stat block (portrait / name / faction / stance / TU+HP bars /
// Wounds pips / wound-name list), reused by BOTH the status panel and the inspect panel — DRY.
mod stat_block;
// Test-support-only re-export of the stat-block's per-widget markers (GTW-278), gated so
// the binary build is `unused`/`unreachable_pub`-clean (the status-panel marker precedent).
#[cfg(feature = "test-support")]
crate::support_use! {
    stat_block::{
        StatFaction, StatHpBar, StatHpLabel, StatName, StatPortrait, StatStance, StatTuBar,
        StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips, portrait_index_for_name,
    };
}

mod status_panel;
pub(in crate::scenes::running::game::battlescape) use status_panel::GameBattleScapeStatusPanelScenePlugin;

// The GTW-274 inspect panel (top-right): the twin of the status panel, rendering the
// shared stat block for the HOVERED ganger / the hovered object's integrity.
mod inspect_panel;
pub(in crate::scenes::running::game::battlescape) use inspect_panel::GameBattleScapeInspectPanelScenePlugin;
// Test-support-only re-export of the inspect-panel's root + object markers (GTW-274), gated
// so the binary build is `unused`/`unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    inspect_panel::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    };
}

// The GTW-275 layout-overhaul BOTTOM BAR: the ONE opaque full-width strip at the bottom of
// the screen — the only UI that reduces the world map (the corner panels are overlays). Its
// root climbs so `set_world_viewport` measures its height for the SOLE viewport inset AND the
// AC tests can assert its presence. The weapon panel sits inside it.
mod bottom_bar;
pub(in crate::scenes::running::game::battlescape) use bottom_bar::GameBattleScapeBottomBarScenePlugin;
crate::support_use!(bottom_bar::BottomBarRoot;);

// The GTW-275 / GTW-298 weapon cluster (bottom-left): the Overall Weapon Panel 2×2 grid
// (Combined weapon + Firemode + Item + Aim) plus the separate Stance Panel. It sits INSIDE the
// bottom bar (GTW-275 overhaul item 6); its root climbs (like the inspect-panel root) AND the AC
// tests can assert its presence.
mod weapon_panel;
pub(in crate::scenes::running::game::battlescape) use weapon_panel::GameBattleScapeWeaponPanelScenePlugin;
// Test-support-only re-export of the weapon-cluster's root + content / name / magazine / reload
// markers + the GTW-298 rework structural markers (Combined / Item / Aim grid cells, image
// placeholder, disabled item buttons), gated so the binary build is `unused`/`unreachable_pub`-
// clean. `WeaponPanelRoot` is test-support-ONLY (the GTW-275 overhaul viewport insets by the
// bottom bar, not the weapon panel — no binary code reads it).
#[cfg(feature = "test-support")]
crate::support_use! {
    weapon_panel::{
        AimLabel, AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage,
        WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
    };
}

// The GTW-294 CONTEXTUAL PANEL (bottom-right): the cluster of situational acts on a downed
// neighbour (Execute / Stabilize / Open Door). This scaffold slice spawns/despawns the panel on
// the `BattleRunning` boundary with all three buttons `Visibility::Hidden` — no behavior yet.
mod contextual_panel;
pub(in crate::scenes::running::game::battlescape) use contextual_panel::ContextualPanelPlugin;
// Test-support-only re-export of the contextual-panel's root + the three button markers (GTW-294),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the weapon-panel marker re-export
// chain precedent). The AC tests name these through `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    contextual_panel::{
        ContextualPanelRoot, ExecuteButton, OpenDoorButton, StabilizeButton,
    };
}
