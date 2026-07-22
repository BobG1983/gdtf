mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameBattleScapeScenePlugin;
mod resources;

// The battlescape sub-state enum lives in the folder it governs (GTW-321); this
// `support_use!` carries it up toward `crate::states::BattleScapeState`.
mod battlescape_state;
crate::support_use!(battlescape_state::BattleScapeState;);

// `pub(crate)` so the crate-root test-support ledger can name the panel `test_support`
// submodules under it (GTW-569 one-hop ledger — markers no longer climb through here).
pub(crate) mod generation;
pub(in crate::states::running::game::battlescape) use generation::GameBattleScapeGenerationScenePlugin;

mod animate_in;
pub(in crate::states::running::game::battlescape) use animate_in::GameBattleScapeAnimateInScenePlugin;

// `pub(crate)` so the crate-root test-support ledger can name the scene's own
// `test_support` submodule directly (GTW-569 one-hop ledger — the `BattleRunningComplete`
// marker no longer climbs through here).
pub(crate) mod battle_running;
pub(in crate::states::running::game::battlescape) use battle_running::GameBattleScapeBattleRunningScenePlugin;

mod animate_out;
pub(in crate::states::running::game::battlescape) use animate_out::GameBattleScapeAnimateOutScenePlugin;

mod aftermath;
pub(in crate::states::running::game::battlescape) use aftermath::GameBattleScapeAfterMathScenePlugin;
// The aftermath sub-state enum climbs from `aftermath` toward
// `crate::states::AfterMathState` (GTW-321 co-location chain).
crate::support_use!(aftermath::AfterMathState;);

// The panel modules below are `pub(crate)` so the crate-root test-support ledger can name
// each panel's own `test_support` submodule directly (GTW-569 one-hop ledger — the panel
// markers no longer climb through here).
pub(crate) mod action_bar;
pub(in crate::states::running::game::battlescape) use action_bar::GameBattleScapeActionBarScenePlugin;

// The GTW-278 shared ganger stat block (portrait / name / faction / stance / TU+HP bars /
// Wounds pips / wound-name list), reused by BOTH the status panel and the inspect panel — DRY.
pub(crate) mod stat_block;

pub(crate) mod status_panel;
pub(in crate::states::running::game::battlescape) use status_panel::GameBattleScapeStatusPanelScenePlugin;

// The GTW-274 inspect panel (top-right): the twin of the status panel, rendering the
// shared stat block for the HOVERED ganger / the hovered object's integrity.
pub(crate) mod inspect_panel;
pub(in crate::states::running::game::battlescape) use inspect_panel::GameBattleScapeInspectPanelScenePlugin;

// The GTW-275 layout-overhaul BOTTOM BAR: the ONE opaque full-width strip at the bottom of
// the screen — the only UI that reduces the world map (the corner panels are overlays). Its
// root climbs so `set_world_viewport` measures its height for the SOLE viewport inset AND the
// AC tests can assert its presence. The weapon panel sits inside it.
mod bottom_bar;
pub(in crate::states::running::game::battlescape) use bottom_bar::GameBattleScapeBottomBarScenePlugin;
crate::support_use!(bottom_bar::BottomBarRoot;);

// The GTW-275 / GTW-298 weapon cluster (bottom-left): the Overall Weapon Panel 2×2 grid
// (Combined weapon + Firemode + Item + Aim) plus the separate Stance Panel. It sits INSIDE the
// bottom bar (GTW-275 overhaul item 6); its root climbs (like the inspect-panel root) AND the AC
// tests can assert its presence.
pub(crate) mod weapon_panel;
pub(in crate::states::running::game::battlescape) use weapon_panel::GameBattleScapeWeaponPanelScenePlugin;

// The GTW-458 SELECTION-CYCLE cluster (bottom-bar far RIGHT): the vertical Prev/Next button
// pair that cycles the SelectedShooter through the player gang in (z,y,x) order (the SAME intents
// Tab / Shift+Tab push — ADR-0001). It sits INSIDE the bottom bar's padding (does not change the
// bar height); its root + button markers climb so the AC tests can assert its presence + width.
pub(crate) mod select_cycle;
pub(in crate::states::running::game::battlescape) use select_cycle::GameBattleScapeSelectCycleScenePlugin;

// The GTW-328 COMBAT-TEXT LOG (bottom-left, ABOVE the weapon panel): the strip of recent combat
// events (movement, shot declarations, hit/miss outcomes, damage/wounds, reloads, turn
// boundaries) that scroll up and fade. It drains the sim combat-event messages + classifies them
// through the shared presenter classifier; its root + line markers climb so the AC test can
// assert the log gains lines + FIFO overflow.
pub(crate) mod combat_log;
pub(in crate::states::running::game::battlescape) use combat_log::GameBattleScapeCombatLogScenePlugin;

// The GTW-294 CONTEXTUAL PANEL (bottom-right): the cluster of situational acts on a downed
// neighbour (Execute / Stabilize / Open Door). This scaffold slice spawns/despawns the panel on
// the `BattleRunning` boundary with all three buttons `Visibility::Hidden` — no behavior yet.
pub(crate) mod contextual_panel;
pub(in crate::states::running::game::battlescape) use contextual_panel::ContextualPanelPlugin;
