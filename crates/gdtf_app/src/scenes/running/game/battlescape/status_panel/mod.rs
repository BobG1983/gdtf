//! The battlescape status HUD panel (GTW-278): a themed `gdtf_ui` panel showing the
//! selected player ganger's shared stat block (portrait / name / faction / stance / TU+HP
//! `ProgressBar`s / Wounds `Pips` / wound-name list — the
//! [`stat_block`](super::stat_block)). The GTW-278 rework of the GTW-252 plain-text panel:
//! the `LifeState` + `Weapon` lines are GONE (user-directed), the name / faction / stance are
//! kept, and the bars / pips / portrait are the shared widgets. UI/view only: it reads the
//! sim's vital components + the input crate's `SelectedShooter`, owns no combat rule, and
//! changes no sim/input.

mod components;
mod plugin;
mod systems;

// GTW-271 — the panel ROOT marker, re-exported to the battlescape neighborhood so the
// `set_world_viewport` system can measure its `ComputedNode` width for the world-map's LEFT
// margin inset (no test-support gating — it stays inside the neighborhood).
pub(in crate::scenes::running::game::battlescape) use components::StatusPanelRoot;
pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeStatusPanelScenePlugin;
